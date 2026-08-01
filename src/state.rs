use crate::board::Board;
use crate::math::*;
use crate::player::Player;
use std::io::Write;

pub struct State {
    pub board: Board,
    pub player: Player,
    pub total_turns: usize,
}
impl State {
    pub fn new(board: Board, player: Player) -> State {
        State {
            board,
            player,
            total_turns: 0,
        }
    }
    /// Clear the screen and draw the board, the player, enemies, everything
    pub fn render(&self) {
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);
        let mut buffer = Vec::new();

        self.board.render_tiles(viewport, &mut buffer);
        self.board.render_enemies(viewport, &mut buffer);
        self.player.render(viewport, &mut buffer);
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
    pub fn increment(&mut self) {
        self.total_turns += 1;
        Board::increment(self);
    }
    pub fn is_reachable(&self, position: Vector<usize>) -> bool {
        todo!()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapObject {
    Player,
    Enemy(crate::board::EnemyID),
    Tile(Vector<usize>),
}
