use std::io::Write;

use crate::input::Input;
use crate::math::*;
use abes_nice_things::Style;

// The text to display for the option and whether or not to quit after
static OPTIONS: &[(&str, fn())] = &[
    ("Play", crate::play),
    ("Settings", crate::settings::settings_editor),
    ("Quit", || unreachable!("Oopsie daisy!")),
];
pub fn main_menu() {
    let mut selector = 0;
    let limit = OPTIONS.len();
    loop {
        render(selector);
        crate::input::weirdify().unwrap();
        match Input::get() {
            Input::Select => {
                if OPTIONS[selector].0 == "Quit" {
                    break;
                }
                crate::input::normalize().unwrap();
                (OPTIONS[selector].1)()
            }
            Input::Walk(direction) | Input::MoveSelector(direction) => match direction {
                Direction::Up => {
                    if selector == 0 {
                        selector = limit;
                    }
                    selector -= 1;
                }
                Direction::Down => {
                    selector += 1;
                    if selector == limit {
                        selector = 0;
                    }
                }
                _ => {}
            },
            _ => continue,
        }
    }
    crate::input::normalize().unwrap();
}
fn render(selector: usize) {
    print!("\x1b[H\x1b[0J");
    for index in 0..OPTIONS.len() {
        if selector == index {
            print!("{}", Style::new().background_green());
        }
        println!("{}\x1b[0m", OPTIONS[index].0)
    }
    print!("\x1b[{};0H", selector + 1);
    std::io::stdout().flush().unwrap();
}
