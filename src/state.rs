use crate::board::Board;
use crate::context_menu::ContextMenu;
use crate::context_menu::ContextMenuID;
use crate::math::*;
use crate::player::Player;
use std::io::Write;

pub struct State {
    pub board: Board,
    pub player: Player,
    pub total_turns: usize,
    pub screen_size: Vector<usize>,
    context_menu_stack: crate::context_menu::Stack,
    /// Whether or not the player is controlling th context menu
    pub context_menu_inputs: bool,
    /// Textual feedback to the player
    pub feedback: String,
    enemy_visuals: [Option<char>; crate::enemy::VTABLES.len()],
    next_enemy_visual: u8,
}
impl State {
    pub fn new(board: Board, player: Player, screen_size: Vector<usize>) -> State {
        State {
            board,
            player,
            total_turns: 0,
            screen_size,
            context_menu_stack: vec![(None, 0, ContextMenuID::default())],
            context_menu_inputs: false,
            feedback: String::new(),
            enemy_visuals: [None; crate::enemy::VTABLES.len()],
            next_enemy_visual: 0,
        }
    }
    /// Clear the screen and draw the board, the player, enemies, everything
    pub fn render(&mut self) {
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);
        let mut buffer = Vec::new();

        self.board.render_tiles(viewport, &mut buffer);
        Board::render_enemies(self, viewport, &mut buffer);
        self.player.render(viewport, &mut buffer);
        self.render_meta_ui(&mut buffer);
        crate::context_menu::ContextMenu::render(self, &mut buffer);

        self.player.position_cursor(viewport, &mut buffer);

        std::io::stdout().write_all(&buffer).unwrap();
        std::io::stdout().flush().unwrap();
    }
    /// Handles the select input (enter) and returns if the turn should be incremented
    pub fn handle_select_input(&mut self) -> bool {
        if self.context_menu_inputs {
            let options = (self.get_context_menu().get_options)(self);
            if options.len() == 0 {
                return false;
            }
            let selector = &mut self.context_menu_stack.last_mut().unwrap().1;
            if *selector >= options.len() {
                *selector = 0;
                return false;
            }
            if !options[*selector].2 {
                return false;
            }
            match &options[*selector].1 {
                crate::context_menu::Choice::Recurse(child, argument_generator) => {
                    let argument = (argument_generator)(self);
                    self.context_menu_stack.push((
                        argument,
                        0,
                        ContextMenuID::new_unchecked(*child),
                    ));
                }
                crate::context_menu::Choice::Act(action) => (action)(self),
            }
            false
        } else {
            const INTERACT_RANGE: usize = 3;
            const SMACK_RANGE: usize = 1;
            let no_range_limit = self.player.no_interact_range_limit;
            if !(no_range_limit
                || self
                    .player
                    .position
                    .is_near(self.player.selector, INTERACT_RANGE))
            {
                return false;
            }

            // TODO: Make it so that players can decide what to interact with on conflict
            let enemy_at_selector = self.board.get_enemy_at_position(self.player.selector);
            if let Some(id) = enemy_at_selector
                && self
                    .player
                    .position
                    .is_near(self.player.selector, SMACK_RANGE)
            {
                Player::attack(self, id);
            // Importantly, you must not be able to close a door while an enemy is on it
            } else if let Some(crate::board::tile::Tile::Door { open, .. }) =
                &mut self.board[self.player.selector]
                && self.player.position != self.player.selector
                && enemy_at_selector.is_none()
            {
                *open = !*open;
            } else {
                return false;
            }
            true
        }
    }
    pub fn handle_toggle_context_menu_input(&mut self) -> bool {
        self.context_menu_inputs ^= true;
        false
    }
    pub fn handle_move_selector_input(&mut self, direction: Direction) -> bool {
        // Context menu shenanigans
        if self.context_menu_inputs {
            let options_len = ContextMenu::get_option_texts(self).len();
            let selector = &mut self.context_menu_stack.last_mut().unwrap().1;
            match direction {
                // Traverse up with overflow
                Direction::Up => {
                    if options_len == 0 {
                        return false;
                    }
                    if *selector == 0 {
                        *selector = options_len;
                    }
                    *selector -= 1;
                }
                // Traverse down with overflow
                Direction::Down => {
                    if options_len == 0 {
                        return false;
                    }
                    *selector += 1;
                    if *selector == options_len {
                        *selector = 0;
                    }
                }
                // unrecurse back up
                Direction::Left => {
                    if self.context_menu_stack.len() > 1 {
                        self.context_menu_stack.pop();
                    }
                }
                // recurse deeper and DON'T run actions
                Direction::Right => {
                    if options_len == 0 {
                        return false;
                    }
                    if let (
                        _,
                        crate::context_menu::Choice::Recurse(child, argument_generator),
                        active,
                    ) = (self.get_context_menu().get_options)(self)
                        [self.context_menu_stack.last().unwrap().1]
                        && active
                    {
                        let argument = (argument_generator)(self);
                        self.context_menu_stack.push((
                            argument,
                            0,
                            ContextMenuID::new_unchecked(child),
                        ));
                    }
                }
            }
        }
        // Normal gameplay
        else {
            Player::handle_move_selector_input(self, direction);
        }
        false
    }
    pub fn increment(&mut self) {
        self.total_turns += 1;
        Board::increment(self);
        Player::increment(self);
    }
    pub fn is_reachable(&self, position: Vector<usize>) -> bool {
        // Make it using the rooms for memoization
        todo!()
    }
    pub fn get_context_menu(&self) -> &'static crate::context_menu::ContextMenu {
        self.context_menu_stack.last().unwrap().2.get_context_menu()
    }
    pub fn get_current_context_menu_id(&self) -> ContextMenuID {
        self.context_menu_stack.last().unwrap().2
    }
    pub fn get_current_context_menu_argument(&self) -> &crate::context_menu::Argument {
        &self.context_menu_stack.last().unwrap().0
    }
    pub fn get_context_menu_selector_mut(&mut self) -> &mut usize {
        &mut self.context_menu_stack.last_mut().unwrap().1
    }
    pub fn render_meta_ui(&self, buffer: &mut impl Write) {
        // all meta ui positions are based on the viewport's height and so are given as offsets
        // 1: feedback
        // 2: health bar
        // 3: energy bar
        // 4: meta info
        // 5: input
        //
        // We don't need to clear this section of screen because drawing the tiles already does
        let base_height = self.board.get_viewport_size().y;
        // feedback
        writeln!(buffer, "\x1b[{};0H{}", base_height + 2, self.feedback).unwrap();

        // health bar
        abes_nice_things::ProgressBar::new(
            self.player.health,
            self.player.max_health,
            (self.board.get_viewport_size().x - 20).min(self.player.max_health),
        )
        .amount_done(true)
        .done_char('#')
        .header_char('#')
        .done_style(*abes_nice_things::Style::new().green().intense(true))
        .draw_to(buffer)
        .unwrap();
        writeln!(buffer).unwrap();

        // energy bar
        abes_nice_things::ProgressBar::new(
            self.player.energy,
            self.player.max_energy,
            (self.board.get_viewport_size().x - 20).min(self.player.max_energy * 5),
        )
        .amount_done(true)
        .done_char('#')
        .header_char('#')
        .done_style(*abes_nice_things::Style::new().cyan().intense(true))
        .draw_to(buffer)
        .unwrap();
        writeln!(buffer).unwrap();

        // meta info
        write!(
            buffer,
            "Selector: {}, Turn: {}, Local turn: {}",
            self.player.selector,
            self.total_turns,
            self.board.get_local_turn(),
        )
        .unwrap();
    }
    pub fn get_input(&self, prompt: String) -> String {
        // First we move to the input row and show the prompt
        print!("\x1b[{};0H{prompt}", self.board.get_viewport_size().y + 6);
        // Then we make the terminal go back to normal
        crate::input::normalize().unwrap();
        // Make sure everything sends
        std::io::stdout().flush().unwrap();
        // Then we get what they typed
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        // Then we make it weird again
        crate::input::weirdify().unwrap();
        // Fuck windows
        buf.pop();
        abes_nice_things::windows!(buf.pop());
        // And return!
        buf
    }
    pub fn get_enemy_char(&mut self, vtable_id: crate::enemy::VTableID) -> char {
        if self
            .player
            .effect_tracker
            .has(crate::effect::EffectID::Confusion)
        {
            return '?';
        }
        let index = vtable_id.to_inner() as usize;
        if let Some(ch) = self.enemy_visuals[index] {
            ch
        } else {
            self.enemy_visuals[index] =
                Some(self.next_enemy_visual.to_string().chars().next().unwrap());
            self.enemy_visuals[index].unwrap()
        }
    }
    pub fn force_render_feedback(&self) {
        print!(
            "\x1b[{};0H{}",
            self.board.get_viewport_size().y + 2,
            self.feedback
        );
        std::io::stdout().flush().unwrap()
    }
}
/// Anything on the board, specifically the player an enemy or a tile
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapObject {
    Player,
    Enemy(crate::board::EnemyID),
    Tile(Vector<usize>),
}
/// Anything with logic: players and enemies
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Entity {
    Player,
    Enemy(crate::board::EnemyID),
}
