use crate::math::Direction;
use crate::math::Vector;
use crate::math::Zone;
use crate::state::State;
use abes_nice_things::Style;
pub struct Player {
    pub position: Vector<usize>,
    pub selector: Vector<usize>,
    render_target: RenderTarget,
}
impl Player {
    pub fn new(spawn: Vector<usize>) -> Player {
        Player {
            position: spawn,
            selector: spawn,
            render_target: RenderTarget::Player,
        }
    }
    pub fn position_cursor(&self, viewport: Zone<usize>) {
        let visual_position = viewport.clamp(self.selector) - viewport.top_left();
        print!("\x1b[{};{}H", visual_position.y + 1, visual_position.x + 1);
    }
    pub fn get_render_target_pos(&self) -> Vector<usize> {
        match self.render_target {
            RenderTarget::Player => self.position,
            RenderTarget::Selector => self.selector,
        }
    }
    /// Tries to move in the given direction, returns true if the turn should be incremented
    pub fn handle_walk_input(state: &mut State, move_dir: Direction) -> bool {
        if !state.board.player_can_move(state.player.position, move_dir) {
            return false;
        }

        state.player.position += move_dir;
        true
    }
    pub fn handle_move_selector_input(state: &mut State, direction: Direction) {
        let viewport = state
            .board
            .calculate_viewport(state.player.get_render_target_pos());
        // It would be an invalid move
        if !state
            .board
            .is_move_on_board(state.player.selector, direction)
        {
            return;
        }
        state.player.selector += direction;
        state.player.selector = viewport.clamp(state.player.selector);
    }
    pub fn handle_change_render_target_input(state: &mut State) {
        state.player.render_target = match state.player.render_target {
            RenderTarget::Player => RenderTarget::Selector,
            RenderTarget::Selector => RenderTarget::Player,
        };
    }
    pub fn render(&self, viewport: Zone<usize>) {
        // Only draw the player if we can see the player
        if viewport.contains(self.position) {
            let visual_pos = self.position - viewport.top_left();
            print!(
                "\x1b[{};{}H{}@\x1b[0m",
                visual_pos.y + 1,
                visual_pos.x + 1,
                Style::new().cyan().intense(true)
            )
        }
    }
}
pub enum RenderTarget {
    Player,
    Selector,
}
