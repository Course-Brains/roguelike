// Base mana cost will be 5
// base radius will be 3
// base damage will be 10-41
// base speed will be 20
//
// radius should increase with root of energy
// damage should increase with energy
// speed should decrease with energy settling at 5 in a reciprocal manner

use super::*;
use crate::board::Projectile;
use crate::board::ProjectileType;
use crate::random::Random;
pub fn cast(state: &mut State, origin: Vector<usize>, target: Vector<usize>, energy_used: usize) {
    let radius = (energy_used as f64).sqrt().ceil() as usize;
    let damage_offset = energy_used * 2;
    let damage_mult = energy_used / 5;
    let damage = damage_offset + ((u8::random() & 32) as usize) * damage_mult;
    let speed = (20.0 - 5.0 * energy_used as f64).max(5.0).ceil() as usize;
    state.board.add_projectile(Projectile::new(
        ProjectileType::Fireball { radius, damage },
        speed,
        origin,
        target,
        true,
    ))
}
