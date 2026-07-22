use crate::math::Direction;
use anyhow::Result;
use std::io::Read;
pub enum Input {
    Walk(Direction),
    MoveSelector(Direction),
    Space,
    Select,
    ChangeRenderTarget,
}
impl Input {
    pub fn get() -> Input {
        let mut stdin = std::io::stdin();
        let mut buf = [0];
        loop {
            stdin.read_exact(&mut buf).unwrap();
            return match buf[0] {
                27 => {
                    stdin.read(&mut buf).unwrap();
                    stdin.read(&mut buf).unwrap();
                    Input::MoveSelector(match buf[0] {
                        b'A' => Direction::Up,
                        b'B' => Direction::Down,
                        b'D' => Direction::Left,
                        b'C' => Direction::Right,
                        _ => continue,
                    })
                }
                b'w' => Input::Walk(Direction::Up),
                b's' => Input::Walk(Direction::Down),
                b'a' => Input::Walk(Direction::Left),
                b'd' => Input::Walk(Direction::Right),
                b' ' => Input::Space,
                b'\n' => Input::Select,
                b't' => Input::ChangeRenderTarget,
                _ => continue,
            };
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
        return Ok(());
    }
    Err(anyhow::Error::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to reset terminal, how did you get this far?",
    )))
}
