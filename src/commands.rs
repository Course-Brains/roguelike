use std::sync::mpsc::{channel, Sender, Receiver};
use std::net::TcpListener;
use crate::{State, Vector};
use albatrice::{ToBinary, FromBinary, Split};
enum Command {
    GetPlayerData,
    SetHealth(usize),
    SetEnergy(usize),
    SetPos(Vector),
    Redraw,
    ListEnemies,
    Kill(usize),
    Spawn(crate::enemy::Variant, Vector)
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
                parse_vector(iter.next(), iter.next())?
            )),
            _ => Err("unknown command".to_string())
        }
    }
    fn enact(self, state: &mut State, out: &mut Sender<String>) {
        match self {
            Command::GetPlayerData => {
                out.send(format!("{:#?}", state.player)).unwrap()
            }
            Command::SetHealth(health) => {
                let prev = state.player.health;
                state.player.health = health;
                out.send(format!("health: {prev} -> {health}")).unwrap();
            }
            Command::SetEnergy(energy) => {
                let prev = state.player.energy;
                state.player.energy = energy;
                out.send(format!("energy: {prev} -> {energy}")).unwrap();
            }
            Command::SetPos(new_pos) => {
                let prev = state.player.pos;
                state.player.pos = new_pos;
                out.send(format!("pos: {prev} -> {new_pos}")).unwrap();
            }
            Command::Redraw => {
                state.render();
            }
            Command::ListEnemies => {
                let mut result = String::new();
                for (index, enemy) in state.board.enemies.iter().enumerate() {
                    result += &format!("{index}: {} at {}\n", enemy.variant, enemy.pos);
                }
                out.send(result).unwrap();
            }
            Command::Kill(index) => {
                out.send(format!("Removed enemy at {}", state.board.enemies.swap_remove(index).pos)).unwrap();
            }
            Command::Spawn(variant, pos) => {
                state.board.enemies.push(crate::enemy::Enemy::new(pos, variant));
            }
        }
    }
}
pub struct CommandHandler {
    rx: Receiver<Command>,
    tx: Sender<String>
}
impl CommandHandler {
    pub fn new() -> CommandHandler {
        let (rx, tx) = listen();
        CommandHandler {
            rx,
            tx
        }
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
        let (mut read, mut write) = TcpListener::bind("127.0.0.1:5287").unwrap().accept().unwrap().0.split().unwrap();
        std::thread::spawn(move || {
            loop {
                match Command::new(String::from_binary(&mut read).unwrap()) {
                    Ok(command) => tx.send(command).unwrap(),
                    Err(error) => error_tx.send(error).unwrap()
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
    <T as std::str::FromStr>::Err: ToString
{
    match option.map(|x| x.parse()) {
        Some(Ok(t)) => Ok(t),
        Some(Err(error)) => Err(error.to_string()),
        None => Err("Expected argument".to_string())
    }
}
fn parse_vector(x: Option<&str>, y: Option<&str>) -> Result<Vector, String> {
    Ok(Vector::new(parse(x)?, parse(y)?))
}
