use crate::board::Board;

pub struct State {
    pub board: Board,
    total_turns: usize,
}
impl State {
    pub fn new(board: Board) -> State {
        State {
            board,
            total_turns: 0,
        }
    }
}
