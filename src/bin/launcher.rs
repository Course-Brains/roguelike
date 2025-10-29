use abes_nice_things::Style;
use std::io::Read;
use std::io::Write;
use std::process::Command;
fn main() {
    let mut choices = vec![
        Choice::new("play", || run(&mut Command::new("./run_script"))),
        Choice::new("stats", || run(Command::new("./run_script").arg("stats"))),
        Choice::new("settings", || {
            run(Command::new("./run_script").arg("settings"))
        }),
        Choice::new("update", || run(Command::new("git").arg("pull"))),
        Choice::new("quit", || false),
    ];
    weirdify();
    let mut index = 0;
    loop {
        render(index, &choices);
        if !process_input(&mut index, &mut choices) {
            break;
        }
    }
}
struct Choice {
    name: &'static str,
    action: Box<dyn Fn() -> bool>,
}
impl Choice {
    fn new(name: &'static str, action: impl Fn() -> bool + 'static) -> Choice {
        Choice {
            name,
            action: Box::new(action),
        }
    }
}
// Returns whether or not to continue the program
fn process_input(index: &mut usize, choices: &mut Vec<Choice>) -> bool {
    let mut buf = [0];
    std::io::stdin().read_exact(&mut buf).unwrap();
    match buf[0] {
        27 => {
            std::io::stdin().read_exact(&mut buf).unwrap();
            std::io::stdin().read_exact(&mut buf).unwrap();
            match buf[0] {
                b'A' => selector_up(index, choices.len()),
                b'B' => selector_down(index, choices.len()),
                _ => {}
            }
        }
        b'w' => selector_up(index, choices.len()),
        b's' => selector_down(index, choices.len()),
        b' ' | b'\n' => {
            normalify();
            let out = (choices[*index].action)();
            if out {
                weirdify()
            }
            return out;
        }
        b'q' => return false,
        #[cfg(debug_assertions)]
        b'D' => {
            // Enabling debug
            choices.push(Choice::new("dialogue", || {
                run(Command::new("./run_script").arg("dialogue"))
            }));
            choices.push(Choice::new("empty", || {
                run(Command::new("./run_script").arg("empty"))
            }))
        }
        _ => {}
    }
    true
}
fn selector_up(index: &mut usize, len: usize) {
    if *index == 0 {
        *index = len;
    }
    *index -= 1;
}
fn selector_down(index: &mut usize, len: usize) {
    *index += 1;
    if *index == len {
        *index = 0;
    }
}
fn render(selected: usize, choices: &Vec<Choice>) {
    // Zero the cursor and clear the screen
    crossterm::queue!(
        std::io::stdout(),
        crossterm::cursor::MoveTo(0, 0),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )
    .unwrap();
    // Draw each line
    for (index, option) in choices.iter().enumerate() {
        if index == selected {
            println!(
                "{}{}{}",
                Style::new().red().intense(true),
                option.name,
                Style::new()
            );
        } else {
            println!("{}", option.name);
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
fn run(command: &mut std::process::Command) -> bool {
    command.spawn().unwrap().wait().unwrap().success()
}
