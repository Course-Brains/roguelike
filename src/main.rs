// Modules
mod board;
mod context_menu;
mod effect;
mod enemy;
mod input;
mod main_menu;
mod math;
mod player;
mod random;
mod raycast;
mod settings;
mod spell;
mod state;
mod upgrade;

use std::io::Write;

use input::Input;
use input::normalize;
use input::weirdify;
use math::Vector;
use math::Zone;

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
    if let Err(error) = std::panic::catch_unwind(main_menu::main_menu) {
        // Panic handling
        let _ = normalize();
        print!("\x1b(B"); // reset confusion
        let _ = std::io::stdout().flush();

        std::panic::panic_any(error)
    }
    // Just because it didn't error doesn't mean we don't want to clean up
    else {
        normalize().unwrap();
        print!("\x1b[(B");
        std::io::stdout().flush().unwrap();
    }
}
fn play() {
    let mut state = state::State::new();

    weirdify().unwrap();
    loop {
        state.render();
        // Yes it does have to be done here
        state.board.reset_took_damage_flags();
        if match Input::get() {
            Input::Walk(direction) => player::Player::handle_walk_input(&mut state, direction),
            Input::MoveSelector(direction) => state.handle_move_selector_input(direction),
            Input::ChangeRenderTarget => {
                player::Player::handle_change_render_target_input(&mut state);
                false
            }
            Input::ToggleContextMenu => state.handle_toggle_context_menu_input(),
            Input::Select => state.handle_select_input(),
            Input::ResetSelectorPosition => {
                state.player.selector = state.player.position;
                false
            }
            Input::SkipTurn => true,
            Input::ResizeScreen => {
                if state.unlocked_settings.resize_trigger_mode().is_manual() {
                    state.rememo_screen_size();
                }
                state.render();
                false
            }
            Input::Number(number) => state.handle_number_input(number),
        } {
            state.increment();
        }
        if state.exit {
            break;
        }
    }
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
fn get_git_hash() -> String {
    String::from_utf8(
        std::process::Command::new("git")
            .arg("log")
            .arg("--oneline")
            .arg("HEAD^..HEAD")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .trim_matches(|ch: char| !ch.is_ascii())
    .split(" ")
    .next()
    .unwrap()
    .to_string()
}
/// Writes the bell character to the given destination or stdout if none is given.
///
/// If it is writing to stdout then it will flush afterwards
fn bell(dest: Option<&mut dyn Write>) -> anyhow::Result<()> {
    match dest {
        Some(dest) => dest.write_all(&[7])?,
        None => {
            let mut stdout = std::io::stdout();
            stdout.write_all(&[7])?;
            stdout.flush()?
        }
    }
    Ok(())
}
enum ThreadAsync<T: Send + 'static> {
    Waiting(std::thread::JoinHandle<T>),
    Done(T),
}
impl<T: Send + 'static> ThreadAsync<T> {
    fn new<F: Fn() -> T + Send + 'static>(func: F) -> Self {
        Self::Waiting(std::thread::spawn(func))
    }
    /// Consumes the [ThreadAsync] and returns the data output by the thread. This is a blocking
    /// operation if the thread is not done
    fn unwrap(self) -> Result<T, Box<dyn std::any::Any + Send + 'static>> {
        match self {
            Self::Waiting(handle) => handle.join(),
            Self::Done(val) => Ok(val),
        }
    }
}
