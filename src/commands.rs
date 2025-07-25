use crate::{Entity, ItemType, Spell, State, Vector, upgrades::UpgradeType};
use albatrice::{FromBinary, Split, ToBinary};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, Sender, channel};
enum Command {
    GetPlayerData,
    SetHealth(Option<usize>),
    SetEnergy(Option<usize>),
    SetPos(Vector),
    Redraw,
    ListEnemies,
    Kill(usize),
    Spawn(crate::enemy::Variant, Option<Vector>),
    GetEnemyData(usize),
    ForceFlood,
    WakeAll,
    OpenAllDoors,
    KillAllEnemies,
    SetPiece(Vector, String),
    LoadNext,
    LoadShop,
    Effect(String),
    Give(ItemType, Option<usize>),
    SetMoney(usize),
    Upgrade(UpgradeType),
    SetDetectMod(isize),
    SetPerception(usize),
    Cast(Spell),
}
impl Command {
    fn new(string: String) -> Result<Command, String> {
        let mut iter = string.split(' ');
        match iter.next().unwrap() {
            "get_player_data" => Ok(Command::GetPlayerData),
            "set_health" => Ok(Command::SetHealth(match iter.next() {
                Some(arg) => Some(parse(Some(arg))?),
                None => None,
            })),
            "set_energy" => Ok(Command::SetEnergy(match iter.next() {
                Some(arg) => Some(parse(Some(arg))?),
                None => None,
            })),
            "set_pos" => Ok(Command::SetPos(parse_vector(iter.next(), iter.next())?)),
            "redraw" => Ok(Command::Redraw),
            "list_enemies" => Ok(Command::ListEnemies),
            "kill" => Ok(Command::Kill(parse(iter.next())?)),
            "spawn" => Ok(Command::Spawn(
                parse(iter.next())?,
                match iter.next() {
                    Some(arg) => Some(Vector::new(parse(Some(arg))?, parse(iter.next())?)),
                    None => None,
                },
            )),
            "get_enemy_data" => Ok(Command::GetEnemyData(parse(iter.next())?)),
            "force_flood" => Ok(Command::ForceFlood),
            "wake_all" => Ok(Command::WakeAll),
            "open_all_doors" => Ok(Command::OpenAllDoors),
            "kill_all_enemies" => Ok(Command::KillAllEnemies),
            "set_piece" => Ok(Command::SetPiece(
                parse_vector(iter.next(), iter.next())?,
                iter.map(|s| s.to_string() + " ").collect(),
            )),
            "load_next" => Ok(Command::LoadNext),
            "load_shop" => Ok(Command::LoadShop),
            "effect" => Ok(Command::Effect(iter.map(|s| s.to_string() + " ").collect())),
            "give" => Ok(Command::Give(
                parse(iter.next())?,
                match iter.next() {
                    Some(arg) => Some(parse(Some(arg))?),
                    None => None,
                },
            )),
            "set_money" => Ok(Command::SetMoney(parse(iter.next())?)),
            "upgrade" => Ok(Command::Upgrade(parse(iter.next())?)),
            "set_detect_mod" => Ok(Command::SetDetectMod(parse(iter.next())?)),
            "set_perception" => Ok(Command::SetPerception(parse(iter.next())?)),
            "cast" => Ok(Command::Cast(
                iter.map(|s| s.to_string() + " ")
                    .collect::<String>()
                    .parse()?,
            )),
            _ => Err("unknown command".to_string()),
        }
    }
    fn enact(self, state: &mut State, out: &mut Sender<String>) {
        match self {
            Command::GetPlayerData => out.send(format!("{:#?}", state.player)).unwrap(),
            Command::SetHealth(health) => {
                state.player.health = health.unwrap_or(state.player.max_health);
            }
            Command::SetEnergy(energy) => {
                state.player.energy = energy.unwrap_or(state.player.max_energy);
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
                        enemy.try_read().unwrap().variant,
                        enemy.try_read().unwrap().pos
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
                        crate::enemy::Enemy::new(pos.unwrap_or(state.player.selector), variant),
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
                    enemy.try_write().unwrap().active = true;
                }
            }
            Command::OpenAllDoors => {
                for piece in state.board.inner.iter_mut() {
                    if let Some(crate::board::Piece::Door(door)) = piece {
                        door.open = true;
                    }
                }
            }
            Command::KillAllEnemies => {
                state.board.enemies = Vec::new();
            }
            Command::SetPiece(pos, args) => {
                state.board[pos] = Some(match args.parse() {
                    Ok(piece) => piece,
                    Err(error) => {
                        out.send(error).unwrap();
                        return;
                    }
                })
            }
            Command::LoadNext => crate::LOAD_MAP.store(true, std::sync::atomic::Ordering::Relaxed),
            Command::LoadShop => crate::LOAD_SHOP.store(true, std::sync::atomic::Ordering::Relaxed),
            Command::Effect(args) => {
                if let Err(msg) = state.player.effects.set(&args) {
                    out.send(msg).unwrap()
                }
            }
            Command::Give(item_type, slot) => {
                let slot = match slot {
                    Some(slot) => slot,
                    None => {
                        // if no slots are open and one hasn't been specified, then use slot zero
                        // and overwrite what is in it
                        let mut out = 0;
                        for slot in 0..6 {
                            if state.player.items[slot].is_none() {
                                out = slot
                            }
                        }
                        out
                    }
                };
                state.player.items[slot] = Some(item_type)
            }
            Command::SetMoney(money) => state.player.money = money,
            Command::Upgrade(upgrade_type) => {
                upgrade_type.on_pickup(&mut state.player);
            }
            Command::SetDetectMod(modifier) => state.player.detect_mod = modifier,
            Command::SetPerception(perception) => state.player.perception = perception,
            Command::Cast(spell) => match spell {
                Spell::Normal(spell) => {
                    let target = state.player.selector;
                    spell.cast(None, &mut state.player, &mut state.board, Some(target));
                }
                Spell::Contact(spell) => {
                    if let Some(enemy) = state.board.get_enemy(state.player.selector, None) {
                        spell.cast(Entity::Enemy(enemy), Entity::Player(&mut state.player));
                    }
                }
            },
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
pub fn parse<T: std::str::FromStr>(option: Option<&str>) -> Result<T, String>
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
