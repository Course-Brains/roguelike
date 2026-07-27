use crate::board::Board;
use crate::math::*;
use crate::player::Player;
use abes_nice_things::{PrimFrom, log};
use std::io::Write;

pub struct State {
    pub board: Board,
    pub player: Player,
    pub total_turns: usize,
}
impl State {
    pub fn new(board: Board, player: Player) -> State {
        State {
            board,
            player,
            total_turns: 0,
        }
    }
    /// Clear the screen and draw the board, the player, enemies, everything
    pub fn render(&self) {
        let center = self.player.get_render_target_pos();
        let viewport = self.board.calculate_viewport(center);

        self.board.render_tiles(viewport);
        self.board.render_enemies(viewport);
        self.player.render(viewport);
        let raycast_result = RayCast::new(self.player.position, self.player.selector)
            .stop_at_target(true)
            .record_path(true)
            .resolve(self);
        for position in raycast_result.1.as_ref().unwrap().iter() {
            print!(
                "\x1b[{};{}H{} \x1b[0m",
                position.y + 1,
                position.x + 1,
                abes_nice_things::Style::new().background_green()
            );
        }
        print!("\x1b[H{:?}", raycast_result.0);
        self.player.position_cursor(viewport);
        std::io::stdout().flush().unwrap();
    }
    /// Handles the select input (enter) and returns if the turn should be incremented
    pub fn handle_select_input(&mut self) -> bool {
        const INTERACT_RANGE: usize = 3;
        const SMACK_RANGE: usize = 1;
        if !self
            .player
            .position
            .is_near(self.player.selector, INTERACT_RANGE)
        {
            return false;
        }

        // TODO: Make it so that players can decide what to interact with on conflict
        let enemy_at_selector = self.board.get_enemy_at_position(self.player.selector);
        if let Some(id) = enemy_at_selector
            && self
                .player
                .position
                .is_near(self.player.selector, SMACK_RANGE)
        {
            Player::attack(self, id);
        // Importantly, you must not be able to close a door while an enemy is on it
        } else if let Some(crate::board::tile::Tile::Door { open, .. }) =
            &mut self.board[self.player.selector]
            && self.player.position != self.player.selector
            && enemy_at_selector.is_none()
        {
            *open = !*open;
        } else {
            return false;
        }
        true
    }
    pub fn increment(&mut self) {
        self.total_turns += 1;
        Board::increment(self);
    }
    pub fn is_reachable(&self, position: Vector<usize>) -> bool {
        todo!()
    }
}
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

        let logical_diff = logical_target - logical_position;
        let mut steps_taken = 0;
        let mut path = if self.record_path {
            Some(Vec::new())
        } else {
            None
        };

        loop {
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
                return (None, path);
            }

            // Figuring out which direction we need to go next
            // figuring out possible next positions
            let next_target = Vector::new(
                if logical_diff.x > 0.0 {
                    (position.x + 1) as f64
                } else if logical_position.x.fract().abs() > 0.0 {
                    position.x as f64
                } else if logical_diff.x == 0.0 {
                    logical_position.x
                } else {
                    (position.x - 1) as f64
                },
                if logical_diff.y > 0.0 {
                    (position.y + 1) as f64
                } else if logical_position.y.fract().abs() > 0.0 {
                    position.y as f64
                } else if logical_diff.y == 0.0 {
                    logical_position.y
                } else {
                    (position.y - 1) as f64
                },
            );

            let effective_dist_to_target = (next_target - logical_position) / logical_diff;
            /*assert!(
                effective_dist_to_target.x.is_sign_positive() || effective_dist_to_target.x == 0.0
            );
            assert!(
                effective_dist_to_target.y.is_sign_positive() || effective_dist_to_target.y == 0.0
            );*/
            // Incrementing everything
            let direction = if effective_dist_to_target.x.abs() < effective_dist_to_target.y.abs()
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
            steps_taken += 1;
            if self.record_path {
                path.as_mut().unwrap().push(position);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapObject {
    Player,
    Enemy(crate::board::EnemyID),
    Tile(Vector<usize>),
}
