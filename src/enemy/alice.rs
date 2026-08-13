use core::index::Last;
use std::{collections::BTreeMap, intrinsics::disjoint_bitor};

use crate::{board::EnemyID, math::{self, Vector}, state::State};
use super::VTable;
pub static VTABLE: VTable = VTable {
    // honestly not really sure how to implement this
    // probably something like with the weight function 
    // with if Some() = map.get_mut("Name")
    // but the "Name" being consistent is a pretty obvious failure method
    starting_health: todo!(),
    is_boss: true,
    init: todo!(),
    think: todo!(),
    damage: todo!(),
    budget_cost: todo!(), // a lot
    tier: todo!(), // no clue
    render_char: todo!(), // needs to be changeable
};

// assumes turn always goes up between thinks
struct History { 
    // everything is an i32 so it works well with the weights
    // since those can be negative
    current_turn: i32, // so things can be relative instead of absolute
    // for keeping track of what's going on 
    // helps decide weights
    last_turn_damaged: i32, // last turn damage was taken
    last_health_lost: i32,
    last_turn_attacked: i32, // last turn damage dealing attack was made
    last_turn_hit: i32, // last turn damage was dealt
    // how many thinks does this character get before the player is in melee range
    // basically just distance, but accounts for energy & being a grid
    thinks_till_player_arrives: i32,
    turns_player_camped: i32, // if it's more than 2 they're probably camping
    // where can the player run away to
    // sorted by distance from player
    player_room_escapes: Vec<Vector<usize>>, // does not include exits to same room as character
}


struct WeightedFragment<'a> {
    fragment: &'a Fragment,
    weight: i32
}
fn weight(history: History, status: FragmentedSoul) {
    let living = status.living_souls;
    // same thing except now it's has a weight field, initialized at 0
    let mut weights = living.iter().map(|fragment_iter|
        (*fragment_iter.0, // the name (key)
            WeightedFragment {
                fragment: fragment_iter.1, // the Fragment(value)
                weight: 0
            }
        )
    ).collect::<BTreeMap<&'static str, WeightedFragment>>(); 
    
    if let Some(alice) = weights.get_mut("Alice") {
        { // damage weighting
            // logarithmically scaled by damage amount, and damage time since
            let health_lost = history.last_health_lost;
            let damage_scalar = (health_lost + 1).ilog2() as i32;
            let turns_since_damaged = history.current_turn - history.last_turn_damaged;
            // tapers off nicely, anything 0 or larger is >=0
            // guaranteed to be positive before the 2 -
            // positive when less then 8 turns, otherwise negative
            let damage_base = 3 - ( (turns_since_damaged + 1).ilog2() as i32 );
            let damage_weight = damage_base * damage_scalar;
            // alice mostly handles pain
            // so her weight increases the more recently it's been
            // tipping point is negative at 8 turns, most positive at 0 turns, most negative when hasn't been hit yet
            // (-29 when damage hasn't been taken yet)
            alice.weight += damage_weight
        }
        { // distance weighting 
            let distance = history.thinks_till_player_arrives;
            // more severe log, since distance is often larger
            // something that is more significant within about 3 or 4 squares and quiclk tapers off would be great
            // but i can't think of fastish math to do that
            // guaranteed to be >=0, and then +1 means the log is guaranteed to also be >=0
            let log_distance = (distance + 1).ilog10() as i32;
            alice.weight += log_distance
        }
    }
    // ok now do that for everyone else too
    // i need to sleep so handing it off to abe to work wit hthe commit while i'm asleep
}

fn death(state: State, id: EnemyID) {
    
}
static DAMAGE_FUNCTION: fn(&mut State, EnemyID, usize) -> bool = {
    |state, id, damage| {
        let this = state.board.get_enemy_mut(id).as_mut().unwrap();
        if damage >= this.health {
            if this.flags.should_general_log() {
                this.log(format!(
                    "Took {damage} damage and died (was at {} health)",
                    this.health
                ));
            }
            *state.board.get_enemy_mut(id) = None;
            return true;
        }
        let prev_health = this.health;
        this.flags.wake();
        this.health -= damage;
        if this.flags.should_general_log() {
            this.log(format!(
                "Took {damage} damage and lost health ({prev_health} -> {})",
                this.health
            ));
        }
        false
    }
};
struct FragmentedSoul {
    living_souls: BTreeMap<&'static str, Fragment> // names
}
#[derive(Clone)]
enum Fragment {
    Snow(),   // debuff and run
    Alice(),  // lots of health, lots of damage, slow
    Rayla(),  // fast, high range, low damage
    Ripley(), // i don't care how small the room is, I CAST FIREBALL
    Jade(),   // traps and slow projectiles
    Echo(),   // copies other enemies, changing upon death
}
