use crate::math::*;
use crate::state::MapObject;
use crate::state::State;
use abes_nice_things::PrimAs;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayCast {
    start: Vector<usize>,
    target: Vector<usize>,
    can_hit_player: bool,
    can_hit_enemy: bool,
    can_hit_tile: bool,
    stop_at_target: bool,
    record_path: bool,
    max_range: Option<usize>,
}
impl RayCast {
    /// Creates a new raycast with some default values, specifically it will not be able to hit
    /// players, can hit enemies and tiles, does not stop upon reaching the target, does not record
    /// its path and does not have a maximum range
    pub const fn new(start: Vector<usize>, target: Vector<usize>) -> RayCast {
        RayCast {
            start,
            target,
            can_hit_player: false,
            can_hit_enemy: true,
            can_hit_tile: true,
            stop_at_target: false,
            record_path: false,
            max_range: None,
        }
    }
    pub fn resolve(self, state: &State) -> (Option<MapObject>, Option<Vec<Vector<usize>>>) {
        let mut path = if self.record_path {
            Some(Vec::new())
        } else {
            None
        };
        let logical_position = self.start.prim_as() + 0.5;
        let logical_target = self.target.prim_as() + 0.5;
        let mut stepper = RayCastStepper {
            position: self.start,
            logical_position,
            target: self.target,
            logical_target,
            steps_taken: 0,
            initial_direction: logical_target - logical_position,
            can_hit_player: self.can_hit_player,
            can_hit_enemy: self.can_hit_enemy,
            can_hit_tile: self.can_hit_tile,
            stop_at_target: self.stop_at_target,
            max_range: self.max_range,
        };

        loop {
            if let Some(result) = stepper.step(state) {
                return (result, path);
            }
            if self.record_path {
                path.as_mut().unwrap().push(stepper.position)
            }
        }
    }
    pub const fn can_hit_player(&mut self, can_hit_player: bool) -> &mut Self {
        self.can_hit_player = can_hit_player;
        self
    }
    pub const fn can_hit_enemy(&mut self, can_hit_enemy: bool) -> &mut Self {
        self.can_hit_enemy = can_hit_enemy;
        self
    }
    pub const fn can_hit_tile(&mut self, can_hit_tile: bool) -> &mut Self {
        self.can_hit_tile = can_hit_tile;
        self
    }
    pub const fn stop_at_target(&mut self, stop_at_target: bool) -> &mut Self {
        self.stop_at_target = stop_at_target;
        self
    }
    pub const fn max_range(&mut self, max_range: Option<usize>) -> &mut Self {
        self.max_range = max_range;
        self
    }
    pub const fn record_path(&mut self, record_path: bool) -> &mut Self {
        self.record_path = record_path;
        self
    }
}
pub struct RayCastStepper {
    // State
    position: Vector<usize>,
    logical_position: Vector<f64>,
    target: Vector<usize>,
    logical_target: Vector<f64>,
    steps_taken: usize,
    initial_direction: Vector<f64>,

    // Settings
    can_hit_player: bool,
    can_hit_enemy: bool,
    can_hit_tile: bool,
    stop_at_target: bool,
    max_range: Option<usize>,
}
impl ToBinary for RayCastStepper {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        abes_nice_things::compact([
            self.can_hit_player,      // 0
            self.can_hit_enemy,       // 1
            self.can_hit_tile,        // 2
            self.stop_at_target,      // 3
            self.max_range.is_some(), // 4
            false,
            false,
            false,
        ])
        .to_binary(binary)?;
        if let Some(max_range) = self.max_range {
            max_range.to_binary(binary)?;
        }
        self.target.to_binary(binary)?;
        self.position.to_binary(binary)?;
        self.logical_position.to_binary(binary)?;
        self.steps_taken.to_binary(binary)?;
        self.initial_direction.to_binary(binary)
    }
}
impl FromBinary for RayCastStepper {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let flags = abes_nice_things::expand(u8::from_binary(binary)?);
        let max_range = if flags[4] {
            Some(usize::from_binary(binary)?)
        } else {
            None
        };
        let target = <Vector<usize>>::from_binary(binary)?;
        Ok(RayCastStepper {
            position: <Vector<usize>>::from_binary(binary)?,
            logical_position: <Vector<f64>>::from_binary(binary)?,
            target,
            logical_target: target.prim_as() + 0.5,
            steps_taken: usize::from_binary(binary)?,
            initial_direction: <Vector<f64>>::from_binary(binary)?,

            can_hit_player: flags[0],
            can_hit_enemy: flags[1],
            can_hit_tile: flags[2],
            stop_at_target: flags[3],
            max_range,
        })
    }
}
impl RayCastStepper {
    /// If it stops then it returns Some(Option<MapObject). If stop_at_target is not enabled then
    /// the inner option will never be None and so you should flatten it.
    pub fn step(&mut self, state: &State) -> Option<Option<MapObject>> {
        // incrementing position
        // complicated thing to find out if we have gone past or are at the target
        let logical_diff = if self.logical_position.x.copysign(self.initial_direction.x)
            >= self.logical_target.x.copysign(self.initial_direction.x)
            || self.logical_position.y.copysign(self.initial_direction.y)
                >= self.logical_target.y.copysign(self.initial_direction.y)
        {
            self.initial_direction
        } else {
            self.logical_target - (self.position.prim_as() + 0.5)
        };
        // Figuring out which direction we need to go next
        // figuring out possible next positions
        let diff_x = logical_diff.x; // pure sugar
        let next_target_x = match diff_x {
            // right is positive x
            _right if diff_x > 0_f64 => {
                // move towards the next integer away from 0
                (self.logical_position.x + 1_f64).floor()
            }
            // left is negative x
            _left if diff_x < 0_f64 => {
                // move to the next integer towards 0
                (self.logical_position.x - 1_f64).ceil()
            }
            _none => {
                // not moving on the x axis at all
                // not actually infinity, in effect it's 0, but expected value later
                f64::INFINITY
            }
        };
        // y ayis
        let diff_y = logical_diff.y; // pure sugar
        let next_target_y = match diff_y {
            // down is positive y
            _down if diff_y > 0_f64 => {
                // move towards the next integer away from 0
                (self.logical_position.y + 1_f64).floor()
            }
            // up is negative y
            _up if diff_y < 0_f64 => {
                // move towards the next integer away from 0
                (self.logical_position.y - 1_f64).ceil()
            }
            _none => {
                // not moving on the y axis at all
                // not actually infinity, in effect it's 0 but expected value later
                f64::INFINITY
            }
        };
        // compose the vector from x and y components
        let next_target = Vector::new(next_target_x, next_target_y);

        let effective_dist_to_target = (next_target - self.logical_position) / logical_diff;
        // Incrementing everything
        if self.position.is_adjacent(self.target) {
            let direction = if self.position.x > self.target.x {
                Direction::Left
            } else if self.position.x < self.target.x {
                Direction::Right
            } else if self.position.y > self.target.y {
                Direction::Up
            } else if self.position.y < self.target.y {
                Direction::Down
            } else {
                unreachable!("We are already at the target")
            };
            self.position += direction;
            assert_eq!(self.position, self.target)
        } else {
            let direction = if effective_dist_to_target.x.abs() < effective_dist_to_target.y.abs()
                && effective_dist_to_target.x.is_finite()
            {
                self.logical_position.x = next_target.x;
                self.logical_position.y += logical_diff.y * effective_dist_to_target.x;
                if logical_diff.x > 0.0 {
                    Direction::Right
                } else {
                    Direction::Left
                }
            } else if effective_dist_to_target.y.is_finite() {
                self.logical_position.y = next_target.y;
                self.logical_position.x += logical_diff.x * effective_dist_to_target.y;
                if logical_diff.y > 0.0 {
                    Direction::Down
                } else {
                    Direction::Up
                }
            } else {
                return Some(None);
            };

            // If moving would take us off the board, then don't
            if !state.board.is_move_on_board(self.position, direction) {
                return Some(None);
            }
            self.position += direction;
            self.steps_taken += 1;
        }
        // Check stop conditions
        // Hitting a player
        if self.can_hit_player && self.position == state.player.position {
            return Some(Some(MapObject::Player));
        }
        // Hitting a tile
        if self.can_hit_tile
            && state.board[self.position].is_some_and(|tile| tile.is_raycast_hittable())
        {
            return Some(Some(MapObject::Tile(self.position)));
        }
        // Hitting an enemy
        if self.can_hit_enemy
            && let Some(enemy) = state.board.get_enemy_at_position(self.position)
        {
            return Some(Some(MapObject::Enemy(enemy)));
        }
        // Hitting the range limit
        if let Some(max_range) = self.max_range
            && self.steps_taken >= max_range
        {
            return Some(None);
        }
        // Hitting the target
        if self.stop_at_target && self.target == self.position {
            return Some(None);
        }
        None
    }
}
