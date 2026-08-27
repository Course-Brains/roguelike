use crate::math::Direction;
use anyhow::Result;
use std::io::Read;
pub enum Input {
    Walk(Direction),
    MoveSelector(Direction),
    ToggleContextMenu,
    Select,
    ChangeRenderTarget,
    SkipTurn,
    ResizeScreen,
    /// The numbers from 1 - 9
    Number(u8),
    ResetSelectorPosition,
    ConvertToCash,
    ToggleDebugMode,
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
                b' ' => Input::ToggleContextMenu,
                b'\n' => Input::Select,
                b't' => Input::ChangeRenderTarget,
                b'r' => Input::ResetSelectorPosition,
                b'c' => Input::ConvertToCash,
                b'\t' => Input::SkipTurn,
                b'+' => Input::ResizeScreen,
                b'1' => Input::Number(1),
                b'2' => Input::Number(2),
                b'3' => Input::Number(3),
                b'4' => Input::Number(4),
                b'5' => Input::Number(5),
                b'6' => Input::Number(6),
                b'7' => Input::Number(7),
                b'8' => Input::Number(8),
                b'9' => Input::Number(9),
                b'?' => Input::ToggleDebugMode,
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
