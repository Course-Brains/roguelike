use super::RenderSpecial;
use crate::math::*;
use crate::raycast::RayCastStepper;
use crate::state::MapObject;
use crate::state::State;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::collections::HashSet;
use std::collections::VecDeque;

////////////////////////////////////////////////////////////
// Things that need changing when adding a new projectile //
////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ProjectileType {
    Fireball { radius: usize, damage: usize },
}
impl ToBinary for ProjectileType {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        match self {
            Self::Fireball { radius, damage } => {
                0_u8.to_binary(binary)?;
                radius.to_binary(binary)?;
                damage.to_binary(binary)
            }
        }
    }
}
impl FromBinary for ProjectileType {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Fireball {
                radius: usize::from_binary(binary)?,
                damage: usize::from_binary(binary)?,
            },
            other => anyhow::bail!(
                "Attempted to create ProjectileType with invalid discriminant: {other}"
            ),
        })
    }
}
impl ProjectileType {
    const HIGHEST_DISCRIMINANT: u8 = 0;
    pub fn render(self) -> (char, Option<Style>) {
        match self {
            Self::Fireball { .. } => ('*', Some(*Style::new().red().intense(true))),
        }
    }
    pub fn on_stop(
        self,
        state: &mut State,
        collision: Option<MapObject>,
        stop_pos: Vector<usize>,
        prev_pos: Vector<usize>,
    ) {
        match self {
            Self::Fireball { radius, damage } => {
                let viewport = state.calculate_viewport();
                // First we get an origin which isn't inside a wall
                let origin = match collision {
                    // The player is a valid origin
                    Some(MapObject::Player) => state.player.position,
                    // Enemies are valid origins
                    Some(MapObject::Enemy(id)) => state.board[id].as_ref().unwrap().get_position(),
                    // Collidable tiles are not
                    Some(MapObject::Tile(_)) => prev_pos,
                    // The target is a valid origin
                    None => stop_pos,
                };

                // Then we flood and render as we flood
                let mut seen = HashSet::from([origin]);
                // The position to visit and the recursive depth to determine when to render
                let mut to_visit = VecDeque::from([(origin, 0)]);
                let mut deepest = 0;
                let explosion_style = *Style::new().background_red().intense_background(true);
                while let Some((current, depth)) = to_visit.pop_front() {
                    if viewport.contains(current) {
                        state.board.short_render_specials.push(RenderSpecial {
                            position: current,
                            ch: ' ',
                            style: Some(explosion_style),
                        });
                    }
                    if current == state.player.position {
                        crate::player::Player::damage(state, damage)
                    } else if let Some(enemy) = state.board.get_enemy_at_position(current) {
                        crate::enemy::Enemy::damage(state, enemy, damage);
                    }
                    if depth > deepest {
                        deepest = depth;
                        state.render();
                        std::thread::sleep(std::time::Duration::from_millis(
                            *state.unlocked_settings.explosion_time(),
                        ))
                    }
                    for dir in Direction::SET.into_iter() {
                        if state.board.is_move_on_board(current, dir)
                            && !state.board[current + dir]
                                .as_ref()
                                .is_some_and(crate::board::Tile::is_player_collidable)
                            && seen.insert(current + dir)
                            && depth < radius
                        {
                            to_visit.push_back((current + dir, depth + 1));
                        }
                    }
                }
                state.board.short_render_specials.truncate(0);
            }
        }
    }
}

////////////////////////
// Things that do not //
////////////////////////

pub struct Projectile {
    pub r#type: ProjectileType,
    pub speed: usize,
    pub stepper: RayCastStepper,
}
impl ToBinary for Projectile {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.r#type.to_binary(binary)?;
        self.speed.to_binary(binary)?;
        self.stepper.to_binary(binary)
    }
}
impl FromBinary for Projectile {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(Projectile {
            r#type: ProjectileType::from_binary(binary)?,
            speed: usize::from_binary(binary)?,
            stepper: RayCastStepper::from_binary(binary)?,
        })
    }
}
impl Projectile {
    pub fn new(
        r#type: ProjectileType,
        speed: usize,
        origin: Vector<usize>,
        target: Vector<usize>,
        stop_at_target: bool,
    ) -> Projectile {
        let mut stepper = *RayCastStepper::new(origin, target)
            .can_hit_player(true)
            .can_hit_enemy(true)
            .can_hit_tile(true);
        if stop_at_target {
            stepper.stop_at_target();
        }
        Self {
            r#type,
            speed,
            stepper,
        }
    }
    /// Move the projectile all the spaces it should move this turn and handle it hitting something
    /// if it does
    ///
    /// It returns if this should be deleted
    pub fn step(state: &mut State, projectile_index: usize, viewport: &Zone<usize>) -> bool {
        // Taking the stepper out of State temporarily so that we can have a mutable reference to
        // both. Just in case it gets put back before the on_stop is run but it shouldn't matter
        //
        // It was either this or cloning the stepper which would be more expensive
        let mut temp_stepper_storage = RayCastStepper::new(Vector::ZERO, Vector::ZERO);
        std::mem::swap(
            &mut temp_stepper_storage,
            &mut state.board.projectiles[projectile_index].stepper,
        );
        for _ in 0..state.board.projectiles[projectile_index].speed {
            let prev_pos = temp_stepper_storage.position();
            if let Some(hit) = temp_stepper_storage.step(state) {
                let stop_pos = temp_stepper_storage.position();
                std::mem::swap(
                    &mut temp_stepper_storage,
                    &mut state.board.projectiles[projectile_index].stepper,
                );
                state.board.projectiles[projectile_index]
                    .r#type
                    .on_stop(state, hit, stop_pos, prev_pos);
                return true;
            }
            if viewport.contains(temp_stepper_storage.position()) {
                std::mem::swap(
                    &mut temp_stepper_storage,
                    &mut state.board.projectiles[projectile_index].stepper,
                );
                state.render();
                std::thread::sleep(std::time::Duration::from_millis(
                    *state.unlocked_settings.projectile_time(),
                ));
                std::mem::swap(
                    &mut temp_stepper_storage,
                    &mut state.board.projectiles[projectile_index].stepper,
                );
            }
        }
        std::mem::swap(
            &mut temp_stepper_storage,
            &mut state.board.projectiles[projectile_index].stepper,
        );
        false
    }
    pub fn position(&self) -> Vector<usize> {
        self.stepper.position()
    }
    pub fn r#type(&self) -> &ProjectileType {
        &self.r#type
    }
}
