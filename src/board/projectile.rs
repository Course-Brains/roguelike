use crate::math::*;
use crate::raycast::RayCast;
use crate::state::MapObject;
use crate::state::State;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;

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
    pub fn on_stop(self, state: &mut State, collision: Option<MapObject>) {
        match self {
            Self::Fireball { radius, damage } => {
                todo!();
            }
        }
    }
}

////////////////////////
// Things that do not //
////////////////////////

pub struct Projectile {
    r#type: ProjectileType,
    position: Vector<usize>,
    logical_position: Vector<f64>,
    speed: usize,
    direction: Vector<f64>,
    /// After how far should it stop and trigger collision code
    travel_limit: Option<usize>,
    target: Vector<usize>,
}
impl ToBinary for Projectile {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.r#type.to_binary(binary)?;
        self.position.to_binary(binary)?;
        self.logical_position.to_binary(binary)?;
        self.speed.to_binary(binary)?;
        self.direction.to_binary(binary)?;
        self.travel_limit.as_ref().to_binary(binary)?;
        self.target.to_binary(binary)
    }
}
impl FromBinary for Projectile {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(Projectile {
            r#type: ProjectileType::from_binary(binary)?,
            position: <Vector<usize>>::from_binary(binary)?,
            logical_position: <Vector<f64>>::from_binary(binary)?,
            speed: usize::from_binary(binary)?,
            direction: <Vector<f64>>::from_binary(binary)?,
            travel_limit: <Option<usize>>::from_binary(binary)?,
            target: <Vector<usize>>::from_binary(binary)?,
        })
    }
}
impl Projectile {
    /// Move the projectile all the spaces it should move this turn and handle it hitting something
    /// if it does
    pub fn step(state: &mut State, projectile_index: usize, viewport: &Zone<usize>) {
        let projectile = &state.board.projectiles[projectile_index];
        let (ground, max_range) = if projectile
            .travel_limit
            .is_some_and(|limit| limit <= projectile.speed)
        {
            (true, projectile.travel_limit.unwrap())
        } else {
            (false, projectile.speed)
        };
        let (hit, path) = RayCast::new(projectile.position, projectile.target)
            .can_hit_player(true)
            .can_hit_enemy(true)
            .can_hit_tile(true)
            .max_range(Some(max_range))
            .record_path(true)
            .resolve(state);
        let path = path.unwrap();
        for position in path.iter().filter(|pos| viewport.contains(**pos)) {
            state.board.projectiles[projectile_index].position = *position;
            state.render();
            std::thread::sleep(std::time::Duration::from_millis(
                *state.unlocked_settings.projectile_time(),
            ));
        }
        if ground || hit.is_some() {
            state.board.projectiles[projectile_index]
                .r#type
                .on_stop(state, hit)
        }
    }
    pub fn position(&self) -> Vector<usize> {
        self.position
    }
    pub fn r#type(&self) -> &ProjectileType {
        &self.r#type
    }
}
