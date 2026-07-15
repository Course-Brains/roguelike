use super::Board;
use crate::Vector;
use abes_nice_things::PrimAs;
use abes_nice_things::Style;
/// Everything which makes up the map itself and not the logic of it, so not enemies but doors and
/// walls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// A wall, you can't walk through it, you can't see through it, you can't shoot through it,
    /// you can't blow it up (it's stronger than you)
    Wall,
    /// Like a wall, but you can make it pretend it doesn't exist. For a while anyway.
    Door {
        open: bool,
    },
    Marker,
}
impl Tile {
    /// Gets the character and optionally the [Style] to draw the tile with
    pub fn render(&self, board: &Board, position: Vector<usize>) -> (char, Option<Style>) {
        match self {
            Tile::Wall => (get_wall_char(board, position), None),
            Tile::Door { open: false } => (get_wall_char(board, position), Some(CLOSED_DOOR_STYLE)),
            Tile::Door { open: true } => OPEN_DOOR,
            Tile::Marker => ('X', Some(*Style::new().purple().background_green())),
        }
    }
    /// Returns if the player will collide with this tile (not be able to walk through it)
    pub fn is_player_collidable(&self) -> bool {
        match self {
            Tile::Wall => true,
            Tile::Door { open } => !open,
            Tile::Marker => false,
        }
    }
    pub fn is_wall_connectable(&self) -> bool {
        true // both walls and doors are always connectable
    }
}
const OPEN_DOOR: (char, Option<Style>) = (WALL_ALL_SIDES, Some(*Style::new().green()));
const CLOSED_DOOR_STYLE: Style = *Style::new().red();
const WALL_ALL_SIDES: char = '╬';
const WALL_T_DOWN: char = '╦';
const WALL_T_UP: char = '╩';
const WALL_T_RIGHT: char = '╠';
const WALL_T_LEFT: char = '╣';
const WALL_CORNER_UP_LEFT: char = '╝';
const WALL_STRAIGHT_VERTICAL: char = '║';
const WALL_CORNER_UP_RIGHT: char = '╚';
const WALL_CORNER_DOWN_LEFT: char = '╗';
const WALL_CORNER_DOWN_RIGHT: char = '╔';
const WALL_STRAIGHT_HORIZONTAL: char = '═';

fn get_wall_char(board: &Board, position: Vector<usize>) -> char {
    let iposition: Vector<isize> = position.prim_as(); // We are not going anywhere near the limit
    let connections = [
        iposition.up(),
        iposition.down(),
        iposition.left(),
        iposition.right(),
    ]
    .map(|position| {
        board
            .try_get_tile(position)
            .is_some_and(|tile| tile.is_some_and(|tile| tile.is_wall_connectable()))
    });
    let up = connections[0];
    let down = connections[1];
    let left = connections[2];
    let right = connections[3];
    let count = up as usize + down as usize + left as usize + right as usize;

    if count == 4 {
        // All sides connected to
        WALL_ALL_SIDES
    } else if count == 3 {
        // One side missing
        if !up {
            WALL_T_DOWN
        } else if !down {
            WALL_T_UP
        } else if !left {
            WALL_T_RIGHT
        } else {
            // !right
            WALL_T_LEFT
        }
    } else if count == 2 {
        // Two missing
        if up {
            if left {
                WALL_CORNER_UP_LEFT
            } else if down {
                WALL_STRAIGHT_VERTICAL
            } else {
                // right
                WALL_CORNER_UP_RIGHT
            }
        } else if down {
            // vertical already handled
            if left {
                WALL_CORNER_DOWN_LEFT
            } else {
                // right
                WALL_CORNER_DOWN_RIGHT
            }
        } else {
            // all cases have been checked for except straight horizontal
            WALL_STRAIGHT_HORIZONTAL
        }
    } else {
        // 0 or 1 connection
        // Invalid wall
        '╳'
    }
}
