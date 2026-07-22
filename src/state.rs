use crate::board::Board;
use crate::player::Player;
use std::io::Write;

pub struct State {
    pub board: Board,
    pub player: Player,
    total_turns: usize,
}
impl State {
    pub fn new(board: Board, player: Player) -> State {
        State {
            board,
            player,
            total_turns: 0,
        }
    }
    pub fn render(&self) {
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);

        self.board.render_tiles(viewport);
        self.board.render_enemies(viewport);
        self.player.render(viewport);
        self.player.position_cursor(viewport);
        std::io::stdout().flush().unwrap();
    }
}
