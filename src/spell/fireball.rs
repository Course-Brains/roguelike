use super::*;
use crate::board::Projectile;
use crate::board::ProjectileType;
use crate::random::Random;

pub fn cast(state: &mut State, origin: Vector<usize>, target: Vector<usize>, energy_used: usize) {
    let damage =
        damage_offset(energy_used) + ((u8::random() & 32) as usize) * damage_mult(energy_used);
    state.board.add_projectile(Projectile::new(
        ProjectileType::Fireball {
            radius: radius(energy_used),
            damage,
        },
        speed(energy_used),
        origin,
        target,
        true,
    ))
}
pub fn get_info(mana: usize) -> Vec<String> {
    vec![
        format!(
            "Damage range: {}-{}",
            damage_offset(mana),
            damage_offset(mana) + 32 * damage_mult(mana)
        ),
        format!("Radius: {}", radius(mana)),
        format!("Proj. speed: {}", speed(mana)),
    ]
}
fn radius(mana: usize) -> usize {
    (mana as f64).sqrt().ceil() as usize
}
fn damage_offset(mana: usize) -> usize {
    mana * 2
}
fn damage_mult(mana: usize) -> usize {
    mana / 5
}
fn speed(mana: usize) -> usize {
    (21.0 - (mana / 3) as f64).max(5.0).ceil() as usize
}
