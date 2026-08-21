use crate::ThreadAsync;
use crate::board::Board;
use crate::board::EnemyID;
use crate::board::map_gen::MapGenSettings;
use crate::context_menu::ContextMenu;
use crate::context_menu::ContextMenuID;
use crate::math::*;
use crate::player::Player;
use crate::settings::LockedSettings;
use crate::settings::UnlockedSettings;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
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
    pub unlocked_settings: UnlockedSettings,
    locked_settings: LockedSettings,
    pub cheats: bool,
    git_hash: String,
    pub next_level: Option<(MapGenSettings, ThreadAsync<Board>)>,
    /// If this is true then the game will quit once we are back in the main event loop
    pub exit: bool,
}
impl ToBinary for State {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.git_hash.to_binary(binary)?;
        self.board.to_binary(binary)?;
        self.player.to_binary(binary)?;
        self.total_turns.to_binary(binary)?;
        // Screen size cannot be usefully saved
        self.context_menu_stack.len().to_binary(binary)?;
        for (argument, index, menu) in self.context_menu_stack.iter() {
            argument.as_ref().to_binary(binary)?;
            index.to_binary(binary)?;
            menu.to_binary(binary)?;
        }
        self.context_menu_inputs.to_binary(binary)?;
        self.feedback.to_binary(binary)?;
        for enemy_visual in self.enemy_visuals.iter() {
            enemy_visual.as_ref().to_binary(binary)?;
        }
        self.next_enemy_visual.to_binary(binary)?;
        // Unlocked settings do not get saved by state
        self.locked_settings.to_binary(binary)?;
        self.cheats.to_binary(binary)?;
        self.next_level
            .as_ref()
            .map(|next| &next.0)
            .to_binary(binary)
    }
}
impl FromBinary for State {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let saved_hash = String::from_binary(binary)?;
        let current_hash = crate::get_git_hash();
        assert_eq!(
            saved_hash, current_hash,
            "The save you tried to load is from a \
        different version of the game and cannot be run on this version:\n\
        Save's version: {}\nCurrent version: {current_hash}",
            saved_hash
        );
        let mut state = State {
            git_hash: saved_hash,
            board: Board::from_binary(binary)?,
            player: Player::from_binary(binary)?,
            total_turns: usize::from_binary(binary)?,
            screen_size: crate::get_terminal_size(),
            context_menu_stack: crate::context_menu::Stack::from_binary(binary)?,
            context_menu_inputs: bool::from_binary(binary)?,
            feedback: String::from_binary(binary)?,
            enemy_visuals: <[Option<char>; crate::enemy::VTABLES.len()]>::from_binary(binary)?,
            next_enemy_visual: u8::from_binary(binary)?,
            unlocked_settings: crate::settings::load_unlocked_settings(),
            locked_settings: crate::settings::LockedSettings::from_binary(binary)?,
            cheats: bool::from_binary(binary)?,
            next_level: None,
            exit: false,
        };
        if let Some(map_gen_settings) = <Option<MapGenSettings>>::from_binary(binary)? {
            state.next_level = Some((
                map_gen_settings,
                crate::ThreadAsync::new(move || {
                    crate::board::map_gen::generate(map_gen_settings).unwrap()
                }),
            ));
        }

        state.finish_load_effects();
        Ok(state)
    }
}
impl State {
    pub fn new() -> State {
        let screen_size = crate::get_terminal_size();
        let (unlocked, locked) = crate::settings::load_from_file();
        let budget =
            ((locked.axis_length().area() / 100) as f32 * locked.enemy_mult()).ceil() as usize;
        let board = crate::board::map_gen::generate(crate::board::map_gen::MapGenSettings::new(
            *locked.axis_length(),
            crate::calc_desired_dimensions(screen_size),
            budget,
        ))
        .unwrap();
        State {
            board,
            player: Player::new(Vector::new(1, 1)),
            total_turns: 0,
            screen_size,
            context_menu_stack: vec![(None, 0, ContextMenuID::default())],
            context_menu_inputs: false,
            feedback: String::new(),
            enemy_visuals: [None; crate::enemy::VTABLES.len()],
            next_enemy_visual: *locked.starting_enemy_char(),
            unlocked_settings: unlocked,
            locked_settings: locked,
            cheats: false,
            git_hash: crate::get_git_hash(),
            next_level: None,
            exit: false,
        }
    }
    /// Clear the screen and draw the board, the player, enemies, everything
    pub fn render(&mut self) {
        if self.unlocked_settings.resize_trigger_mode().is_auto() {
            self.rememo_screen_size();
        }
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);
        let mut buffer = Vec::new();

        self.board.render_tiles(viewport, &mut buffer);
        Board::render_enemies(self, viewport, &mut buffer);
        self.player.render(viewport, &mut buffer);
        self.board.render_projectiles(viewport, &mut buffer);
        self.board.render_short_specials(viewport, &mut buffer);
        self.render_meta_ui(&mut buffer);
        crate::context_menu::ContextMenu::render(self, &mut buffer);

        self.player.position_cursor(viewport, &mut buffer);

        self.set_cursor_shape(&mut buffer);

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
                    let argument = argument_generator
                        .as_ref()
                        .map(|generator| (generator)(self));
                    self.context_menu_stack
                        .push((argument, 0, ContextMenuID::new(*child)));
                }
                crate::context_menu::Choice::Act(action) => (action)(self),
                crate::context_menu::Choice::Info => {}
            }
            false
        } else {
            if self.player.is_dead() {
                self.exit = true;
                return false;
            }
            const SMACK_RANGE: usize = 1;
            if !self.player.within_interact_range(self.player.selector) {
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
                    ) = &(self.get_context_menu().get_options)(self)
                        [self.context_menu_stack.last().unwrap().1]
                        && *active
                    {
                        let argument = argument_generator
                            .as_ref()
                            .map(|generator| (generator)(self));
                        self.context_menu_stack
                            .push((argument, 0, ContextMenuID::new(*child)));
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
    pub fn handle_number_input(&mut self, number: u8) -> bool {
        if number
            <= (self
                .get_current_context_menu_id()
                .get_context_menu()
                .get_options)(self)
            .len() as u8
        {
            *self.get_context_menu_selector_mut() = number as usize - 1;
        }
        false
    }
    pub fn set_cursor_shape(&mut self, buffer: &mut impl Write) {
        // Currently the only thing that would require setting the cursor to an underscore is enemy
        // windup so we can just see if there is an enemy with a background under the cursor
        if self
            .board
            .get_enemy_at_position(self.player.selector)
            .is_some_and(|id| crate::enemy::Enemy::render(self, id).1.has_background())
        {
            crate::cursor_blink(Some(buffer)).unwrap();
        } else {
            crate::reset_cursor_shape(Some(buffer)).unwrap()
        }
    }
    pub fn increment(&mut self) {
        // If the player is dead then changing the game state becomes illegal
        if self.player.is_dead() {
            return;
        }
        self.total_turns += 1;
        Board::increment(self);
        Player::increment(self);
    }
    pub fn get_context_menu(&self) -> &'static crate::context_menu::ContextMenu {
        self.context_menu_stack.last().unwrap().2.get_context_menu()
    }
    pub fn get_current_context_menu_id(&self) -> ContextMenuID {
        self.context_menu_stack.last().unwrap().2
    }
    pub fn get_current_context_menu_argument(&self) -> &Option<crate::context_menu::Argument> {
        &self.context_menu_stack.last().unwrap().0
    }
    pub fn get_context_menu_selector_mut(&mut self) -> &mut usize {
        &mut self.context_menu_stack.last_mut().unwrap().1
    }
    pub fn set_current_context_menu_argument(&mut self, new: crate::context_menu::Argument) {
        let current = &mut self.context_menu_stack.last_mut().unwrap().0;
        *current = Some(new);
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
            self.player.health(),
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
            "Selector: {}, Turn: {}",
            self.player.selector, self.total_turns,
        )
        .unwrap();
    }
    pub fn get_input(&self, prompt: &str) -> String {
        // First we move to the input row and show the prompt
        print!("\x1b[{};0H{prompt}", self.board.get_viewport_size().y + 6);
        std::io::stdout().flush().unwrap();
        // Then we make the terminal go back to normal
        crate::input::normalize().unwrap();
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
    /// This uses a mapper to determine if it should pass along the mapped value or try getting
    /// input again but it will exit and return None if it is given quit, stop, back, lemme out,
    /// or nuh uh
    ///
    /// If the mapper decides to reject the given text then it will send the bell character
    pub fn get_input_with_mapper<T>(
        &self,
        prompt: &str,
        mapper: impl Fn(String) -> Option<T>,
    ) -> Option<T> {
        loop {
            let input = self.get_input(prompt);
            if matches!(
                input.to_lowercase().as_str(),
                "quit" | "stop" | "back" | "lemme out" | "nuh uh"
            ) {
                return None;
            }
            if let Some(out) = (mapper)(input) {
                return Some(out);
            } else {
                crate::bell(None).unwrap();
            }
        }
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
    /// Run all on_starts for all active effects which must be reran on load
    ///
    /// This MUST be run on load and no other times
    pub fn finish_load_effects(&mut self) {
        for effect in crate::effect::EffectTracker::iter_effect_ids() {
            if self.player.effect_tracker.has(effect) && effect.needs_on_start_rerun_on_load() {
                effect.force_run_on_start(self, Entity::Player)
            }
        }
    }
    pub fn locked_settings(&self) -> &crate::settings::LockedSettings {
        &self.locked_settings
    }
    pub fn go_to_shop(&mut self) {
        // First we start the map gen
        let budget = self.calculate_budget();
        let map_gen_settings = MapGenSettings::new(
            *self.locked_settings().axis_length(),
            crate::calc_desired_dimensions(self.screen_size),
            budget,
        );
        self.next_level = Some((
            map_gen_settings,
            crate::ThreadAsync::new(move || {
                crate::board::map_gen::generate(map_gen_settings).unwrap()
            }),
        ));

        // Then we make the room
        self.board = Board::create_blank_shop(crate::calc_desired_dimensions(self.screen_size));

        let axis_length = self.board.axis_length();
        // Then we fill it with goodies
        self.board[Vector::new(axis_length.to_inner() - 2, 1)] = Some(
            crate::board::tile::Tile::WalkTrigger(crate::board::tile::WalkTrigger::Exit),
        );

        // And we do some housekeeping
        self.player.position = Vector::new(1, 1);
        self.player.selector = Vector::new(1, 1);
    }
    /// This is only to be used while currently in a completed level
    fn calculate_budget(&self) -> usize {
        // Math:
        // budget = board_area/100 + total_turns/100

        // We start with a base of map area / 100
        let mut budget = self.board.axis_length().area() / 100;

        // We add total turns / 100
        budget += self.total_turns / 100;

        // And we multiply by the multiplier
        budget = (budget as f32 * self.locked_settings().enemy_mult()).ceil() as usize;

        budget
    }
    pub fn rememo_screen_size(&mut self) {
        self.screen_size = crate::get_terminal_size();
        let desired = crate::calc_desired_dimensions(self.screen_size);
        self.board.recalc_viewport(desired);
    }
    pub fn calculate_viewport(&self) -> Zone<usize> {
        self.board
            .calculate_viewport(self.player.get_render_target_pos())
    }
}

/// Anything on the board, specifically the player an enemy or a tile
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapObject {
    Player,
    Enemy(crate::board::EnemyID),
    Tile(Vector<usize>),
}
impl ToBinary for MapObject {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            MapObject::Player => 0_u8.to_binary(binary),
            MapObject::Enemy(id) => {
                1_u8.to_binary(binary)?;
                id.to_binary(binary)
            }
            MapObject::Tile(pos) => {
                2_u8.to_binary(binary)?;
                pos.to_binary(binary)
            }
        }
    }
}
impl FromBinary for MapObject {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match u8::from_binary(binary)? {
            0 => MapObject::Player,
            1 => MapObject::Enemy(crate::board::EnemyID::from_binary(binary)?),
            2 => MapObject::Tile(<Vector<usize>>::from_binary(binary)?),
            _ => {
                return Err(anyhow::Error::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get MapObject from binary due to invalid discriminant",
                )));
            }
        })
    }
}

/// Anything with logic: players and enemies
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Entity {
    Player,
    Enemy(crate::board::EnemyID),
}
impl ToBinary for Entity {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            Entity::Player => false.to_binary(binary),
            Entity::Enemy(id) => {
                true.to_binary(binary)?;
                id.to_binary(binary)
            }
        }
    }
}
impl FromBinary for Entity {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match bool::from_binary(binary)? {
            false => Entity::Player,
            true => Entity::Enemy(crate::board::EnemyID::from_binary(binary)?),
        })
    }
}
impl Entity {
    pub fn is_player(&self) -> bool {
        matches!(self, Entity::Player)
    }
    pub fn is_enemy(&self) -> bool {
        matches!(self, Entity::Enemy(_))
    }
}
impl From<EnemyID> for Entity {
    fn from(value: EnemyID) -> Self {
        Entity::Enemy(value)
    }
}
