use crate::{State, Vector};
use albatrice::{FromBinary, Split, ToBinary};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, Sender, channel};
enum Command {
    GetPlayerData,
    SetHealth(usize),
    SetEnergy(usize),
    SetPos(Vector),
    Redraw,
    ListEnemies,
    Kill(usize),
    Spawn(crate::enemy::Variant, Vector),
    GetEnemyData(usize),
    ForceFlood,
    WakeAll,
    OpenAllDoors,
}
impl Command {
    fn new(string: String) -> Result<Command, String> {
        let mut iter = string.split(' ');
        match iter.next().unwrap() {
            "get_player_data" => Ok(Command::GetPlayerData),
            "set_health" => Ok(Command::SetHealth(parse(iter.next())?)),
            "set_energy" => Ok(Command::SetEnergy(parse(iter.next())?)),
            "set_pos" => Ok(Command::SetPos(parse_vector(iter.next(), iter.next())?)),
            "redraw" => Ok(Command::Redraw),
            "list_enemies" => Ok(Command::ListEnemies),
            "kill" => Ok(Command::Kill(parse(iter.next())?)),
            "spawn" => Ok(Command::Spawn(
                parse(iter.next())?,
                parse_vector(iter.next(), iter.next())?,
            )),
            "get_enemy_data" => Ok(Command::GetEnemyData(parse(iter.next())?)),
            "force_flood" => Ok(Command::ForceFlood),
            "wake_all" => Ok(Command::WakeAll),
            "open_all_doors" => Ok(Command::OpenAllDoors),
            _ => Err("unknown command".to_string()),
        }
    }
    fn enact(self, state: &mut State, out: &mut Sender<String>) {
        match self {
            Command::GetPlayerData => out.send(format!("{:#?}", state.player)).unwrap(),
            Command::SetHealth(health) => {
                state.player.health = health;
            }
            Command::SetEnergy(energy) => {
                state.player.energy = energy;
            }
            Command::SetPos(new_pos) => {
                state.player.pos = new_pos;
            }
            Command::Redraw => {
                state.render();
            }
            Command::ListEnemies => {
                let mut result = String::new();
                for (index, enemy) in state.board.enemies.iter().enumerate() {
                    result += &format!(
                        "{index}: {} at {}\n",
                        enemy.read().unwrap().variant,
                        enemy.read().unwrap().pos
                    );
                }
                out.send(result).unwrap();
            }
            Command::Kill(index) => {
                state.board.enemies.swap_remove(index);
            }
            Command::Spawn(variant, pos) => {
                state
                    .board
                    .enemies
                    .push(std::sync::Arc::new(std::sync::RwLock::new(
                        crate::enemy::Enemy::new(pos, variant),
                    )));
            }
            Command::GetEnemyData(index) => {
                out.send(format!("{:#?}", state.board.enemies[index]))
                    .unwrap();
            }
            Command::ForceFlood => {
                crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Command::WakeAll => {
                for enemy in state.board.enemies.iter_mut() {
                    enemy.write().unwrap().active = true;
                }
            }
            Command::OpenAllDoors => {
                for piece in state.board.inner.iter_mut() {
                    if let Some(crate::board::Piece::Door(door)) = piece {
                        door.open = true;
                    }
                }
            }
        }
    }
}
pub struct CommandHandler {
    rx: Receiver<Command>,
    tx: Sender<String>,
}
impl CommandHandler {
    pub fn new() -> CommandHandler {
        let (rx, tx) = listen();
        CommandHandler { rx, tx }
    }
    pub fn handle(&mut self, state: &mut State) {
        while let Ok(command) = self.rx.try_recv() {
            command.enact(state, &mut self.tx)
        }
    }
}
fn listen() -> (Receiver<Command>, Sender<String>) {
    let (tx, rx) = channel();
    let (tx_out, rx_out) = channel::<String>();
    let error_tx = tx_out.clone();
    std::thread::spawn(|| {
        let (mut read, mut write) = TcpListener::bind("127.0.0.1:5287")
            .unwrap()
            .accept()
            .unwrap()
            .0
            .split()
            .unwrap();
        std::thread::spawn(move || {
            loop {
                match Command::new(String::from_binary(&mut read).unwrap()) {
                    Ok(command) => tx.send(command).unwrap(),
                    Err(error) => error_tx.send(error).unwrap(),
                }
            }
        });
        std::thread::spawn(move || {
            loop {
                rx_out.recv().unwrap().to_binary(&mut write).unwrap();
            }
        });
    });
    (rx, tx_out)
}
fn parse<T: std::str::FromStr>(option: Option<&str>) -> Result<T, String>
where
    <T as std::str::FromStr>::Err: ToString,
{
    match option.map(|x| x.parse()) {
        Some(Ok(t)) => Ok(t),
        Some(Err(error)) => Err(error.to_string()),
        None => Err("Expected argument".to_string()),
    }
}
fn parse_vector(x: Option<&str>, y: Option<&str>) -> Result<Vector, String> {
    Ok(Vector::new(parse(x)?, parse(y)?))
}
