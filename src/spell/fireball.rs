use super::*;
use crate::board::Projectile;
use crate::board::ProjectileType;
pub fn cast(state: &mut State, origin: Vector<usize>, target: Vector<usize>, energy_used: usize) {
    state.board.add_projectile(Projectile::new(
        ProjectileType::Fireball {
            radius: 5,
            damage: 5,
        },
        5,
        origin,
        target,
        true,
    ))
}
