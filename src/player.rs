use crate::math::Direction;
use crate::math::Vector;
use crate::math::Zone;
use crate::state::Entity;
use crate::state::State;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::io::Write;

pub struct Player {
    pub position: Vector<usize>,
    pub selector: Vector<usize>,
    render_target: RenderTarget,
    health: usize,
    pub max_health: usize,
    pub energy: usize,
    pub max_energy: usize,
    pub effect_tracker: crate::effect::EffectTracker,
    pub flags: PlayerFlags,
    killer: Option<Entity>,
}
impl ToBinary for Player {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.position.to_binary(binary)?;
        self.selector.to_binary(binary)?;
        self.render_target.to_binary(binary)?;
        self.health.to_binary(binary)?;
        self.max_health.to_binary(binary)?;
        self.energy.to_binary(binary)?;
        self.max_energy.to_binary(binary)?;
        self.effect_tracker.to_binary(binary)?;
        self.flags.to_binary(binary)?;
        self.killer.as_ref().to_binary(binary)
    }
}
impl FromBinary for Player {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(Player {
            position: <Vector<usize>>::from_binary(binary)?,
            selector: <Vector<usize>>::from_binary(binary)?,
            render_target: RenderTarget::from_binary(binary)?,
            health: usize::from_binary(binary)?,
            max_health: usize::from_binary(binary)?,
            energy: usize::from_binary(binary)?,
            max_energy: usize::from_binary(binary)?,
            effect_tracker: crate::effect::EffectTracker::from_binary(binary)?,
            flags: PlayerFlags::from_binary(binary)?,
            killer: <Option<Entity>>::from_binary(binary)?,
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
            effect_tracker: Default::default(),
            flags: Default::default(),
            killer: None,
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
        if state.player.is_dead() {
            return false;
        }
        if !state.board.player_can_move(state.player.position, move_dir) {
            // There is something blocking movement
            if let Some(id) = state
                .board
                .get_enemy_at_position(state.player.position + move_dir)
                && *state.unlocked_settings.kick_enemies()
            {
                Player::attack(state, id);
                return true;
            } else if let Some(crate::board::tile::Tile::Door { open, .. }) =
                &mut state.board[state.player.position + move_dir]
                && !*open
                && *state.unlocked_settings.kick_doors()
            {
                *open = true;
                return true;
            }
            return false;
        }

        state.player.position += move_dir;
        let pos = state.player.position;
        if let Some(crate::board::tile::Tile::WalkTrigger(walk_trigger)) = &state.board[pos] {
            let walk_trigger = walk_trigger.clone();
            if walk_trigger.handle_player(state) {
                state.board[pos] = None;
            }
        }
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
            // If we are dead then the player is greyed out
            let style = if self.is_alive() {
                *Style::new().cyan().intense(true)
            } else {
                Style::new()
            };
            write!(
                buffer,
                "\x1b[{};{}H{}@\x1b[0m",
                visual_pos.y + 1,
                visual_pos.x + 1,
                style
            )
            .unwrap();
        }
    }
    /// The function for damaging the player. It properly handles things so only use this.
    ///
    /// A source of None is to show the player damaging themself
    pub fn damage(state: &mut State, damage: usize, source: Entity) {
        let player = &mut state.player;
        // If the player is dead then there is no point doing furthur damage
        if player.is_dead() {
            return;
        }

        player.health = player.health.saturating_sub(damage);

        // If the player has died then label that
        if player.health() == 0 {
            state.feedback = "You have died. Press enter to exit.".to_string();
            crate::bell(Some(&mut std::io::stdout())).unwrap();
            player.killer = Some(source)
        }
    }
    pub fn health(&self) -> usize {
        self.health
    }
    pub fn increment(state: &mut State) {
        let finished = state.player.effect_tracker.decriment();
        crate::effect::EffectTracker::run_on_ends(state, crate::state::Entity::Player, finished);
    }
    pub fn is_dead(&self) -> bool {
        self.killer.is_some()
    }
    pub fn is_alive(&self) -> bool {
        self.killer.is_none()
    }
    pub fn get_killer(&self) -> Option<Entity> {
        self.killer
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    Player,
    Selector,
}
impl ToBinary for RenderTarget {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            RenderTarget::Player => false,
            RenderTarget::Selector => true,
        }
        .to_binary(binary)
    }
}
impl FromBinary for RenderTarget {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match bool::from_binary(binary)? {
            false => RenderTarget::Player,
            true => RenderTarget::Selector,
        })
    }
}
pub struct PlayerFlags {
    dead: bool,
    no_interact_range_limit: bool,
}
impl ToBinary for PlayerFlags {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.dead.to_binary(binary)?;
        self.no_interact_range_limit.to_binary(binary)
    }
}
impl FromBinary for PlayerFlags {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(PlayerFlags {
            dead: bool::from_binary(binary)?,
            no_interact_range_limit: bool::from_binary(binary)?,
        })
    }
}
impl Default for PlayerFlags {
    fn default() -> Self {
        Self {
            dead: false,
            no_interact_range_limit: false,
        }
    }
}
impl PlayerFlags {
    pub fn no_interact_range_limit(&self) -> bool {
        self.no_interact_range_limit
    }
    pub fn swap_no_interact_range_limit(&mut self) {
        self.no_interact_range_limit ^= true;
    }
}
