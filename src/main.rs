// Modules
mod board;
mod context_menu;
mod enemy;
mod input;
mod math;
mod player;
mod random;
mod raycast;
mod state;

use abes_nice_things::log;
use board::AxisLength;
use input::Input;
use input::normalize;
use input::weirdify;
use math::Vector;
use math::Zone;
use raycast::RayCast;

// Visual space allocation is
// vvvvvvvvvvvvvvvv Viewport
// +--------------++-----+<
// |              ||     |<
// |              ||     |<
// |              ||     |<
// |              ||     |< Context menu
// +--------------+|     |<
// +--------------++-----+<
// ^^^^^^^^^^^^^^^^ Bars/meta ui

fn main() {
    abes_nice_things::set_log_path("log").expect("Failed to set log path");
    if let Err(error) = std::panic::catch_unwind(run) {
        // Panic handling
        let _ = normalize();

        std::panic::panic_any(error)
    }
}
fn run() {
    let terminal_size = get_terminal_size();
    let mut state = state::State::new(
        board::map_gen::generate(
            AxisLength::Full,
            calc_desired_dimensions(terminal_size),
            10000,
        )
        .unwrap(),
        player::Player::new(Vector::new(1, 1)),
        terminal_size,
    );

    weirdify().unwrap();
    loop {
        state.render();
        if match Input::get() {
            Input::Walk(direction) => player::Player::handle_walk_input(&mut state, direction),
            Input::MoveSelector(direction) => state.handle_move_selector_input(direction),
            Input::ChangeRenderTarget => {
                player::Player::handle_change_render_target_input(&mut state);
                false
            }
            Input::ToggleContextMenu => state.handle_toggle_context_menu_input(),
            Input::Select => {
                state.handle_select_input()
                /*if state.board.count_enemies() == 0 {
                    state.board.add_enemy(enemy::Enemy::new(
                        &enemy::dummy::VTABLE,
                        state.player.selector,
                    ));
                } else {
                    state
                        .board
                        .get_enemy_mut(board::EnemyID(0))
                        .as_mut()
                        .unwrap()
                        .move_target = Some(state.player.selector);
                }*/
            }
            Input::DebugQuery => {
                log!("Debug query for position: {}", state.player.selector);
                log!("  turn: {}", state.total_turns);
                log!("  The tile is: {:?}", state.board[state.player.selector]);
                if let Some(id) = state.board.get_enemy_at_position(state.player.selector) {
                    log!("  There is an enemy ({}): {:#?}", id.0, state.board[id]);
                } else {
                    log!("  There is no enemy at that position");
                }
                if state.player.position == state.player.selector {
                    log!("  The player is there");
                }
                log!(
                    "  The possible rooms it is a part of are: {:?}",
                    state
                        .board
                        .get_possible_room_ids_at_position(state.player.selector)
                );

                false
            }
        } {
            state.increment();
        }
    }
    //normalize().unwrap();
}
/// Calculates the desired width, height for the viewport. It gets the terminal's size then
/// subtracts the areas needed for other parts of the ui. If the resulting viewport would be too
/// small then it panics.
///
/// When using this to create a [Zone] for the viewport, remember to subtract 1 from the width and
/// height first because [Zone]s are inclusive.
fn calc_desired_dimensions(mut screen_size: Vector<usize>) -> Vector<usize> {
    // Viewport border
    screen_size -= 1;

    // bars/meta ui:
    //  feedback
    //  health
    //  energy
    //  meta info
    //  input
    screen_size.y -= 5;

    // Right column
    screen_size.x -= context_menu::COLUMNS_NEEDED;

    // validity checks
    if screen_size.x < 20 {
        panic!("Terminal is under minimum width")
    }
    if screen_size.y < 10 {
        panic!("terminal is under minimum height")
    }
    screen_size
}
/// Gets the size of the terminal in width, height.
///
/// This takes about 10ms independant of whether it is release or debug.
fn get_terminal_size() -> Vector<usize> {
    // These get the width and height respectively, the reason why they have to inherit stderr is
    // because they ask stderr what size it is
    Vector::new(
        String::from_utf8(
            std::process::Command::new("tput")
                .arg("cols")
                .stderr(std::process::Stdio::inherit())
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .parse()
        .expect("This NEEDS stderr to be the terminal in order to work"),
        String::from_utf8(
            std::process::Command::new("tput")
                .arg("lines")
                .stderr(std::process::Stdio::inherit())
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .parse()
        .expect("This NEEDS stderr to be the terminal in order to work"),
    )
}
