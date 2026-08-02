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
    pub context_menu_selector: usize,
    context_menu: ContextMenuID,
    /// Whether or not the player is controlling th context menu
    pub context_menu_inputs: bool,
}
impl State {
    pub fn new(board: Board, player: Player, screen_size: Vector<usize>) -> State {
        State {
            board,
            player,
            total_turns: 0,
            screen_size,
            context_menu_stack: Vec::new(),
            context_menu_selector: 0,
            context_menu: ContextMenuID::default(),
            context_menu_inputs: false,
        }
    }
    /// Clear the screen and draw the board, the player, enemies, everything
    pub fn render(&mut self) {
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);
        let mut buffer = Vec::new();

        self.board.render_tiles(viewport, &mut buffer);
        self.board.render_enemies(viewport, &mut buffer);
        self.player.render(viewport, &mut buffer);
        crate::context_menu::ContextMenu::render(self, &mut buffer);

        self.player.position_cursor(viewport, &mut buffer);

        std::io::stdout().write_all(&buffer).unwrap();
        std::io::stdout().flush().unwrap();
    }
    /// Handles the select input (enter) and returns if the turn should be incremented
    pub fn handle_select_input(&mut self) -> bool {
        const INTERACT_RANGE: usize = 3;
        const SMACK_RANGE: usize = 1;
        if !self
            .player
            .position
            .is_near(self.player.selector, INTERACT_RANGE)
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
    pub fn handle_toggle_context_menu_input(&mut self) -> bool {
        self.context_menu_inputs ^= true;
        false
    }
    pub fn handle_move_selector_input(&mut self, direction: Direction) -> bool {
        // Context menu shenanigans
        if self.context_menu_inputs {
            let options_len = ContextMenu::get_option_texts(self).len();
            match direction {
                // Traverse up with overflow
                Direction::Up => {
                    if options_len == 0 {
                        return false;
                    }
                    if self.context_menu_selector == 0 {
                        self.context_menu_selector = options_len;
                    }
                    self.context_menu_selector -= 1;
                }
                // Traverse down with overflow
                Direction::Down => {
                    if options_len == 0 {
                        return false;
                    }
                    self.context_menu_selector += 1;
                    if self.context_menu_selector == options_len {
                        self.context_menu_selector = 0;
                    }
                }
                // unrecurse back up
                Direction::Left => {
                    if let Some(parent) = self.get_context_menu().get_parent() {
                        self.context_menu = parent;
                        self.context_menu_stack.pop();
                    }
                }
                // recurse deeper and DON'T run actions
                Direction::Right => {
                    if options_len == 0 {
                        return false;
                    }
                    if let (_, crate::context_menu::Choice::Recurse(child, argument_generator)) =
                        (self.get_context_menu().get_options)(self)[self.context_menu_selector]
                    {
                        let argument = (argument_generator)(self);
                        self.context_menu_stack.push(argument);
                        self.context_menu = ContextMenuID::new_unchecked(child);
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
    }
    pub fn is_reachable(&self, position: Vector<usize>) -> bool {
        // Make it using the rooms for memoization
        todo!()
    }
    pub fn get_context_menu(&self) -> &'static crate::context_menu::ContextMenu {
        self.context_menu.get_context_menu()
    }
    pub fn get_current_context_menu_argument(&self) -> Option<&crate::context_menu::Argument> {
        self.context_menu_stack.last()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapObject {
    Player,
    Enemy(crate::board::EnemyID),
    Tile(Vector<usize>),
}
