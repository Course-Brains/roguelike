use abes_nice_things::{FromBinary, ToBinary, input};
use std::io::Read;
fn main() {
    let mut write = std::fs::OpenOptions::new()
        .write(true)
        .open("console_in")
        .unwrap();
    println!("in fifo open");
    let mut read = std::fs::File::open("console_out").unwrap();
    println!("out fifo open");
    println!("Connection made");
    if std::env::args().any(|arg| &arg == "--auto" || &arg == "-a")
        && let Ok(mut file) = std::fs::File::open("command_auto_run")
    {
        println!("Running command script commands");
        let mut string = String::new();
        file.read_to_string(&mut string).unwrap();
        for command in string.lines() {
            let command = command.trim();
            if command.is_empty() {
                continue;
            }
            if command.starts_with("//") {
                continue;
            }
            println!("Running: ({command})");
            command.to_string().to_binary(&mut write).unwrap();
        }
    }
    std::thread::spawn(move || while input().to_binary(&mut write).is_ok() {});
    while let Ok(string) = String::from_binary(&mut read) {
        println!("{string}");
    }
}
