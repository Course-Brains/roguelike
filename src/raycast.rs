use crate::math::*;
use crate::state::MapObject;
use crate::state::State;
use abes_nice_things::PrimAs;
use abes_nice_things::PrimFrom;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
impl ToBinary for RayCast {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        // We are storing if max range exists with the other bools because it saves space
        self.start.to_binary(binary)?;
        self.target.to_binary(binary)?;
        abes_nice_things::compact([
            self.can_hit_player,      // 0
            self.can_hit_enemy,       // 1
            self.can_hit_tile,        // 2
            self.stop_at_target,      // 3
            self.record_path,         // 4
            self.max_range.is_some(), // 5
            false,
            false,
        ])
        .to_binary(binary)?;
        if let Some(max_range) = self.max_range {
            max_range.to_binary(binary)?;
        }
        Ok(())
    }
}
impl FromBinary for RayCast {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let start = <Vector<usize>>::from_binary(binary)?;
        let target = <Vector<usize>>::from_binary(binary)?;
        let bools = abes_nice_things::expand(u8::from_binary(binary)?);
        Ok(RayCast {
            start,
            target,
            can_hit_player: bools[0],
            can_hit_enemy: bools[1],
            can_hit_tile: bools[2],
            stop_at_target: bools[3],
            record_path: bools[4],
            max_range: if bools[5] {
                Some(usize::from_binary(binary)?)
            } else {
                None
            },
        })
    }
}
impl RayCast {
    /// Creates a new raycast with some default values, specifically it will not be able to hit
    /// players, can hit enemies and tiles, does not stop upon reaching the target, does not record
    /// its path and does not have a maximum range
    pub fn new(start: Vector<usize>, target: Vector<usize>) -> RayCast {
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
        let mut logical_position = Vector::<f64>::prim_from(self.start) + 0.5;
        let mut position = self.start;
        let logical_target = Vector::<f64>::prim_from(self.target) + 0.5;
        let initial_direction = logical_target - logical_position;

        let mut steps_taken = 0;
        let mut path = if self.record_path {
            Some(Vec::new())
        } else {
            None
        };

        loop {
            let logical_diff = if steps_taken >= self.start.abs_diff(self.target).sum_axes() {
                initial_direction
            } else {
                logical_target - (position.prim_as() + 0.5)
            };
            // Check stop conditions
            // Hitting a player
            if self.can_hit_player && position == state.player.position {
                return (Some(MapObject::Player), path);
            }
            // Hitting a tile
            if self.can_hit_tile
                && state.board[position].is_some_and(|tile| tile.is_raycast_hittable())
            {
                return (Some(MapObject::Tile(position)), path);
            }
            // Hitting an enemy
            if self.can_hit_enemy
                && let Some(enemy) = state.board.get_enemy_at_position(position)
            {
                return (Some(MapObject::Enemy(enemy)), path);
            }
            // Hitting the range limit
            if let Some(max_range) = self.max_range
                && steps_taken >= max_range
            {
                return (None, path);
            }
            // Hitting the target
            if self.stop_at_target && self.target == position {
                assert_eq!(steps_taken, self.start.abs_diff(self.target).sum_axes());
                return (None, path);
            }
            let mut x_style = abes_nice_things::Style::new();
            let mut y_style = x_style.clone();
            // Figuring out which direction we need to go next
            // figuring out possible next positions
            let diff_x = logical_diff.x; // pure sugar
            let next_target_x = match diff_x {
                // right is positive x
                _right if diff_x > 0_f64 => {
                    x_style.background_green();
                    // move towards the next integer away from 0
                    (logical_position.x + 1_f64).floor()
                }
                // left is negative x
                _left if diff_x < 0_f64 => {
                    // move to the next integer towards 0
                    x_style.background_blue();
                    (logical_position.x - 1_f64).ceil()
                }
                _none => {
                    // not moving on the x axis at all
                    // not actually infinity, in effect it's 0, but expected value later
                    x_style.background_cyan();
                    f64::INFINITY
                }
            };
            // y ayis
            let diff_y = logical_diff.y; // pure sugar
            let next_target_y = match diff_y {
                // down is positive y
                _down if diff_y > 0_f64 => {
                    // move towards the next integer away from 0
                    y_style.background_red();
                    (logical_position.y + 1_f64).floor()
                }
                // up is negative y
                _up if diff_y < 0_f64 => {
                    // move towards the next integer away from 0
                    y_style.background_yellow();
                    (logical_position.y - 1_f64).ceil()
                }
                _none => {
                    // not moving on the y axis at all
                    // not actually infinity, in effect it's 0 but expected value later
                    y_style.background_purple();
                    f64::INFINITY
                }
            };
            // compose the vector from x and y components
            let next_target = Vector::new(next_target_x, next_target_y);

            let effective_dist_to_target = (next_target - logical_position) / logical_diff;
            /*assert!(
                effective_dist_to_target.x.is_sign_positive() || effective_dist_to_target.x == 0.0
            );
            assert!(
                effective_dist_to_target.y.is_sign_positive() || effective_dist_to_target.y == 0.0
            );*/
            // Incrementing everything
            if position.is_adjacent(self.target) {
                let direction = if position.x > self.target.x {
                    Direction::Left
                } else if position.x < self.target.x {
                    Direction::Right
                } else if position.y > self.target.y {
                    Direction::Up
                } else if position.y < self.target.y {
                    Direction::Down
                } else {
                    unreachable!("We are already at the target")
                };
                print!(
                    "\x1b[{};{}H{} \x1b[0m",
                    position.y + 1,
                    position.x + 1,
                    abes_nice_things::Style::new().background_purple()
                );
                position += direction;
                steps_taken += 1;
                if self.record_path {
                    path.as_mut().unwrap().push(position)
                }
                assert_eq!(position, self.target)
            } else {
                let direction = if effective_dist_to_target.x.abs()
                    < effective_dist_to_target.y.abs()
                    && effective_dist_to_target.x.is_finite()
                {
                    logical_position.x = next_target.x;
                    logical_position.y += logical_diff.y * effective_dist_to_target.x;
                    if logical_diff.x > 0.0 {
                        Direction::Right
                    } else {
                        Direction::Left
                    }
                } else if effective_dist_to_target.y.is_finite() {
                    logical_position.y = next_target.y;
                    logical_position.x += logical_diff.x * effective_dist_to_target.y;
                    if logical_diff.y > 0.0 {
                        Direction::Down
                    } else {
                        Direction::Up
                    }
                } else {
                    return (None, path);
                };

                // If moving would take us off the board, then don't
                if !state.board.is_move_on_board(position, direction) {
                    return (None, path);
                }
                position += direction;
                match direction.axis() {
                    Axis::Horizontal => {
                        print!(
                            "\x1b[{};{}H{} \x1b[0m",
                            position.y + 1,
                            position.x + 1,
                            x_style
                        )
                    }
                    Axis::Vertical => {
                        print!(
                            "\x1b[{};{}H{} \x1b[0m",
                            position.y + 1,
                            position.x + 1,
                            y_style
                        )
                    }
                }
                steps_taken += 1;
                if self.record_path {
                    path.as_mut().unwrap().push(position);
                }
            }
        }
    }
    pub fn can_hit_player(&mut self, can_hit_player: bool) -> &mut Self {
        self.can_hit_player = can_hit_player;
        self
    }
    pub fn can_hit_enemy(&mut self, can_hit_enemy: bool) -> &mut Self {
        self.can_hit_enemy = can_hit_enemy;
        self
    }
    pub fn can_hit_tile(&mut self, can_hit_tile: bool) -> &mut Self {
        self.can_hit_tile = can_hit_tile;
        self
    }
    pub fn stop_at_target(&mut self, stop_at_target: bool) -> &mut Self {
        self.stop_at_target = stop_at_target;
        self
    }
    pub fn max_range(&mut self, max_range: Option<usize>) -> &mut Self {
        self.max_range = max_range;
        self
    }
    pub fn record_path(&mut self, record_path: bool) -> &mut Self {
        self.record_path = record_path;
        self
    }
}
