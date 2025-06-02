use std::sync::mpsc::{channel, Sender, Receiver};
use std::net::TcpListener;
use crate::State;
use albatrice::{ToBinary, FromBinary};
enum Command {
    GetPlayerData,
    SetHealth(usize),
    SetEnergy(usize),
}
impl Command {
    fn new(string: String) -> Result<Command, String> {
        let mut iter = string.split(' ');
        match iter.next().unwrap() {
            "get_player_data" => Ok(Command::GetPlayerData),
            "set_health" => Ok(Command::SetHealth(parse(iter.next())?)),
            "set_energy" => Ok(Command::SetEnergy(parse(iter.next())?)),
            _ => Err("unknown command".to_string())
        }
    }
    fn enact(self, state: &mut State, out: &mut Sender<String>) {
        match self {
            Command::GetPlayerData => {
                out.send(format!("{:#?}", state.player)).unwrap()
            }
            Command::SetHealth(health) => state.player.health = health,
            Command::SetEnergy(energy) => state.player.energy = energy,
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
        if let Ok(command) = self.rx.try_recv() {
            command.enact(state, &mut self.tx)
        }
    }
}
fn listen() -> (Receiver<Command>, Sender<String>) {
    let (tx, rx) = channel();
    let (tx_out, rx_out) = channel::<String>();
    std::thread::spawn(move || {
        let mut stream = TcpListener::bind("127.0.0.1:5287").unwrap().accept().unwrap().0;
        loop {
            match Command::new(String::from_binary(&mut stream).unwrap()) {
                Ok(command) => tx.send(command).unwrap(),
                Err(error) => error.to_binary(&mut stream).unwrap()
            }
            rx_out.recv().unwrap().to_binary(&mut stream).unwrap();
        }
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
