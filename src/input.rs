use crate::vector::Direction;
use anyhow::Result;
use std::io::Read;
pub enum Input {
    Direction(Direction),
}
impl Input {
    pub fn get() -> Input {
        let mut stdin = std::io::stdin();
        let mut buf = [0];
        loop {
            stdin.read_exact(&mut buf).unwrap();
            match buf[0] {
                27 => {
                    stdin.read(&mut buf).unwrap();
                    stdin.read(&mut buf).unwrap();
                    match buf[0] {
                        b'A' => return Input::Direction(Direction::Up),
                        b'B' => return Input::Direction(Direction::Down),
                        b'D' => return Input::Direction(Direction::Left),
                        b'C' => return Input::Direction(Direction::Right),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}

static IS_WEIRD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn weirdify() -> Result<()> {
    if IS_WEIRD.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }
    if std::process::Command::new("stty")
        .arg("-echo")
        .arg("-icanon")
        .status()?
        .success()
    {
        print!("\x1b[?25l");
        return Ok(());
    }
    Err(anyhow::Error::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to modify terminal, are you on windows?",
    )))
}
pub fn normalize() -> Result<()> {
    if !IS_WEIRD.swap(false, std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }
    if std::process::Command::new("stty")
        .arg("echo")
        .arg("icanon")
        .status()?
        .success()
    {
        print!("\x1b[?25h");
        return Ok(());
    }
    Err(anyhow::Error::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to reset terminal, how did you get this far?",
    )))
}
