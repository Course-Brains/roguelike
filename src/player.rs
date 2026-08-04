use crate::math::Direction;
use crate::math::Vector;
use crate::math::Zone;
use crate::state::State;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};
use std::io::Write;

pub struct Player {
    pub position: Vector<usize>,
    pub selector: Vector<usize>,
    render_target: RenderTarget,
    pub health: usize,
    pub max_health: usize,
    pub energy: usize,
    pub max_energy: usize,
    pub no_interact_range_limit: bool,
}
impl ToBinary for Player {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.position.to_binary(binary)?;
        self.selector.to_binary(binary)?;
        self.render_target.to_binary(binary)?;
        self.health.to_binary(binary)?;
        self.max_health.to_binary(binary)?;
        self.energy.to_binary(binary)?;
        self.max_energy.to_binary(binary)?;
        self.no_interact_range_limit.to_binary(binary)
    }
}
impl FromBinary for Player {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Player {
            position: <Vector<usize>>::from_binary(binary)?,
            selector: <Vector<usize>>::from_binary(binary)?,
            render_target: RenderTarget::from_binary(binary)?,
            health: usize::from_binary(binary)?,
            max_health: usize::from_binary(binary)?,
            energy: usize::from_binary(binary)?,
            max_energy: usize::from_binary(binary)?,
            no_interact_range_limit: bool::from_binary(binary)?,
        })
    }
}
impl Player {
    pub fn new(spawn: Vector<usize>) -> Player {
        Player {
            position: spawn,
            selector: spawn,
            render_target: RenderTarget::Player,
            health: 50,
            max_health: 100,
            energy: 3,
            max_energy: 5,
            no_interact_range_limit: false,
        }
    }
    pub fn position_cursor(&self, viewport: Zone<usize>, buffer: &mut impl Write) {
        let visual_position = viewport.clamp(self.selector) - viewport.top_left();
        write!(
            buffer,
            "\x1b[{};{}H",
            visual_position.y + 1,
            visual_position.x + 1
        )
        .unwrap();
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
            // There is something blocking movement
            if let Some(id) = state
                .board
                .get_enemy_at_position(state.player.position + move_dir)
            {
                Player::attack(state, id);
                return true;
            }
            return false;
        }

        state.player.position += move_dir;
        true
    }
    pub fn attack(state: &mut State, target: crate::board::EnemyID) {
        (state.board[target].as_ref().unwrap().get_vtable().damage)(state, target, 1);
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
    pub fn render(&self, viewport: Zone<usize>, buffer: &mut impl Write) {
        // Only draw the player if we can see the player
        if viewport.contains(self.position) {
            let visual_pos = self.position - viewport.top_left();
            write!(
                buffer,
                "\x1b[{};{}H{}@\x1b[0m",
                visual_pos.y + 1,
                visual_pos.x + 1,
                Style::new().cyan().intense(true)
            )
            .unwrap();
        }
    }
    pub fn damage(state: &mut State, damage: usize) {
        state.player.health = state.player.health.saturating_sub(damage);
    }
}
pub enum RenderTarget {
    Player,
    Selector,
}
impl ToBinary for RenderTarget {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            RenderTarget::Player => false,
            RenderTarget::Selector => true,
        }
        .to_binary(binary)
    }
}
impl FromBinary for RenderTarget {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match bool::from_binary(binary)? {
            false => RenderTarget::Player,
            true => RenderTarget::Selector,
        })
    }
}
