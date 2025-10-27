use abes_nice_things::Style;
use std::io::Read;
use std::io::Write;
use std::process::Command;
static OPTIONS: [&'static str; 5] = ["play", "stats", "settings", "update", "quit"];
fn main() {
    weirdify();
    let mut index = 0;
    loop {
        render(index);
        if !process_input(&mut index) {
            break;
        }
    }
}
// Returns whether or not to continue the program
fn enact_chosen(index: usize) -> bool {
    match index {
        0 => {
            // play
            Command::new("./run_script")
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success()
        }
        1 => {
            // stats
            Command::new("./run_script")
                .arg("stats")
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success()
        }
        2 => {
            // settings
            Command::new("./run_script")
                .arg("settings")
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success()
        }
        3 => {
            // update
            Command::new("git")
                .arg("pull")
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success()
        }
        4 => {
            // quit
            false
        }
        _ => unreachable!("Tell Course-Brains to fix his code"),
    }
}
// Returns whether or not to continue the program
fn process_input(index: &mut usize) -> bool {
    let mut buf = [0];
    std::io::stdin().read_exact(&mut buf).unwrap();
    match buf[0] {
        27 => {
            std::io::stdin().read_exact(&mut buf).unwrap();
            std::io::stdin().read_exact(&mut buf).unwrap();
            match buf[0] {
                b'A' => selector_up(index),
                b'B' => selector_down(index),
                _ => {}
            }
        }
        b'w' => selector_up(index),
        b's' => selector_down(index),
        b' ' | b'\n' => {
            normalify();
            let out = enact_chosen(*index);
            if out {
                weirdify()
            }
            return out;
        }
        _ => {}
    }
    true
}
fn selector_up(index: &mut usize) {
    if *index == 0 {
        *index = OPTIONS.len();
    }
    *index -= 1;
}
fn selector_down(index: &mut usize) {
    *index += 1;
    if *index == OPTIONS.len() {
        *index = 0;
    }
}
fn render(selected: usize) {
    // Zero the cursor and clear the screen
    crossterm::queue!(
        std::io::stdout(),
        crossterm::cursor::MoveTo(0, 0),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();
    for (index, option) in OPTIONS.iter().enumerate() {
        if index == selected {
            println!(
                "{}{option}{}",
                Style::new().red().intense(true),
                Style::new()
            );
        } else {
            println!("{option}");
        }
    }
    std::io::stdout().flush().unwrap()
}
fn weirdify() {
    // send input immediately and don't echo input to user
    std::process::Command::new("stty")
        .arg("-icanon")
        .arg("-echo")
        .status()
        .unwrap();
    crossterm::execute!(std::io::stdout(), crossterm::cursor::Hide).unwrap();
}
fn normalify() {
    std::process::Command::new("stty")
        .arg("icanon")
        .arg("echo")
        .status()
        .unwrap();
    crossterm::execute!(std::io::stdout(), crossterm::cursor::Show).unwrap();
}
