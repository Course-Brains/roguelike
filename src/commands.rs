use crate::{
    Entity, ItemType, Player, Spell, State, Style, Vector, spell::SpellCircle,
    upgrades::UpgradeType,
};
use albatrice::{FromBinary, Split, ToBinary};
use std::net::TcpListener;
use std::sync::mpsc::{Receiver, Sender, channel};
enum Command {
    GetPlayerData,
    SetHealth(Option<usize>),
    SetEnergy(Option<usize>),
    SetPos(SmartVector),
    Redraw,
    ListEnemies,
    Kill(usize),
    Spawn(crate::enemy::Variant, SmartVector),
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
    CreateCircle(Spell, SmartVector, SmartVector),
    GetData(SmartVector),
    GetBoss,
    CountEnemies,
    Checksum,
    SetBench(bool),
    EnableLog(usize, String),
}
impl Command {
    fn new(string: String) -> Result<Command, String> {
        crate::CHEATS.store(true, crate::Ordering::Relaxed);
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
            "set_pos" => Ok(Command::SetPos(SmartVector::new(
                iter.next(),
                iter.next(),
                true,
            )?)),
            "redraw" => Ok(Command::Redraw),
            "list_enemies" => Ok(Command::ListEnemies),
            "kill" => Ok(Command::Kill(parse(iter.next())?)),
            "spawn" => Ok(Command::Spawn(
                parse(iter.next())?,
                SmartVector::new(iter.next(), iter.next(), true)?,
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
            "create_circle" => Ok(Command::CreateCircle(
                (arg(iter.next())?.to_string() + " " + arg(iter.next())?).parse()?,
                SmartVector::new(iter.next(), iter.next(), false)?,
                SmartVector::new(iter.next(), iter.next(), true)?,
            )),
            "get_data" => Ok(Command::GetData(SmartVector::new(
                iter.next(),
                iter.next(),
                true,
            )?)),
            "get_boss" => Ok(Command::GetBoss),
            "count_enemies" => Ok(Command::CountEnemies),
            "checksum" => Ok(Command::Checksum),
            "set_bench" => Ok(Command::SetBench(
                iter.next()
                    .unwrap_or("true")
                    .parse::<bool>()
                    .map_err(|e| e.to_string())?,
            )),
            "enable_log" => Ok(Command::EnableLog(parse(iter.next())?, parse(iter.next())?)),
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
                state.player.pos = new_pos.to_absolute(&state.player);
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
                        crate::enemy::Enemy::new(pos.to_absolute(&state.player), variant),
                    )));
                crate::re_flood();
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
                for piece in state.board.inner.iter_mut().flatten() {
                    if let crate::board::Piece::Door(door) = piece {
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
            Command::SetMoney(money) => unsafe { *state.player.mut_money() = money },
            Command::Upgrade(upgrade_type) => {
                upgrade_type.on_pickup(&mut state.player);
            }
            Command::SetDetectMod(modifier) => state.player.detect_mod = modifier,
            Command::SetPerception(perception) => state.player.perception = perception,
            Command::Cast(spell) => match spell {
                Spell::Normal(spell) => {
                    let target = state.player.selector;
                    spell.cast(
                        None,
                        &mut state.player,
                        &mut state.board,
                        None,
                        Some(target),
                        None,
                    );
                }
                Spell::Contact(spell) => {
                    if let Some(enemy) = state.board.get_enemy(state.player.selector, None) {
                        spell.cast(Entity::Enemy(enemy), Entity::Player(&mut state.player));
                    }
                }
            },
            Command::CreateCircle(spell, pos, aim) => state.board.spells.push(SpellCircle {
                spell,
                pos: pos.to_absolute(&state.player),
                caster: None,
                aim: Some(aim.to_absolute(&state.player)),
            }),
            Command::GetData(pos) => {
                let pos = pos.to_absolute(&state.player);
                let index = state.board.to_index(pos);
                if state.board.seen[index] {
                    out.send("It has been seen".to_string()).unwrap();
                }
                if state.board.visible[index] {
                    out.send("It is within the visibility flood".to_string())
                        .unwrap();
                } else if state.is_visible(pos) {
                    out.send("It is visible".to_string()).unwrap()
                }
                if state.board.reachable[index] {
                    out.send("It is reachable".to_string()).unwrap();
                }
                if let Some(piece) = &state.board[pos] {
                    out.send(format!("It is a {piece}")).unwrap();
                }
                if state.player.pos == pos {
                    out.send("It contains the player".to_string()).unwrap();
                }
                for (index, enemy) in state.board.enemies.iter().enumerate() {
                    if enemy.try_read().unwrap().pos == pos {
                        out.send(format!(
                            "It contains enemy {}, which is a {}",
                            index,
                            enemy.try_read().unwrap().variant
                        ))
                        .unwrap();
                    }
                }
                for special in state.board.specials.iter() {
                    if let Some(special) = special.upgrade() {
                        if special.pos == pos {
                            out.send(format!(
                                "It has a special with visuals: {}{}\x1b0m",
                                special.style.unwrap_or(Style::new()),
                                special.ch
                            ))
                            .unwrap();
                        }
                    }
                }
            }
            Command::GetBoss => match state.board.boss.as_ref().map(|weak| weak.upgrade()) {
                Some(Some(arc)) => {
                    let mut index = None;
                    for (ind, enemy) in state.board.enemies.iter().enumerate() {
                        if std::sync::Arc::ptr_eq(&arc, enemy) {
                            index = Some(ind);
                        }
                    }
                    let enemy = arc.try_read().unwrap();
                    out.send(format!(
                        "The boss is a: {} at pos: {} and index: {}",
                        enemy.variant,
                        enemy.pos,
                        index.unwrap()
                    ))
                    .unwrap()
                }
                _ => out.send("There is no boss".to_string()).unwrap(),
            },
            Command::CountEnemies => {
                let mut count = std::collections::HashMap::new();
                for enemy in state.board.enemies.iter() {
                    let variant_num = match enemy.try_read().unwrap().variant {
                        crate::enemy::Variant::Basic => 0,
                        crate::enemy::Variant::BasicBoss(_) => 1,
                        crate::enemy::Variant::Mage(_) => 2,
                        crate::enemy::Variant::MageBoss(_) => 3,
                        crate::enemy::Variant::Fighter(_) => 4,
                        crate::enemy::Variant::FighterBoss { .. } => 5,
                    };
                    let prev = count.get(&variant_num).unwrap_or(&0);
                    count.insert(variant_num, prev + 1);
                }
                out.send(format!("There are {} basic", count.get(&0).unwrap_or(&0)))
                    .unwrap();
                out.send(format!(
                    "There are {} basic_boss",
                    count.get(&1).unwrap_or(&0)
                ))
                .unwrap();
                out.send(format!("There are {} mage", count.get(&2).unwrap_or(&0)))
                    .unwrap();
                out.send(format!(
                    "There are {} mage_boss",
                    count.get(&3).unwrap_or(&0)
                ))
                .unwrap();
                out.send(format!("There are {} fighter", count.get(&4).unwrap_or(&0)))
                    .unwrap();
                out.send(format!(
                    "There are {} fighter_boss",
                    count.get(&5).unwrap_or(&0)
                ))
                .unwrap();
            }
            Command::Checksum => {
                let mut failed = false;
                for enemy in state.board.enemies.iter() {
                    let pos = enemy.try_read().unwrap().pos;
                    if state.board[pos].is_some() {
                        out.send(format!("Failure at {pos}")).unwrap();
                        failed = true;
                    }
                }
                if !failed {
                    out.send("no faults".to_string()).unwrap()
                }
            }
            Command::SetBench(state) => {
                if state {
                    crate::enable_benchmark();
                } else {
                    crate::bench::BENCHMARK.store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
            Command::EnableLog(index, path) => {
                state.board.enemies[index].try_write().unwrap().log =
                    Some(std::fs::File::create(path).unwrap());
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
#[derive(Clone, Copy)]
enum Pos {
    Absolute(usize),
    RelativePlayer(isize),
    RelativeSelector(isize),
}
impl Pos {
    fn to_absolute(self, player: usize, selector: usize) -> usize {
        match self {
            Pos::Absolute(pos) => pos,
            Pos::RelativePlayer(offset) => (player as isize + offset) as usize,
            Pos::RelativeSelector(offset) => (selector as isize + offset) as usize,
        }
    }
}
impl std::str::FromStr for Pos {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // relative to player
        if s.starts_with('p') {
            Ok(Pos::RelativePlayer(err_to_string(
                s.trim_start_matches('p').parse(),
            )?))
        }
        // relative to selector
        else if s.starts_with('s') {
            Ok(Pos::RelativeSelector(err_to_string(
                s.trim_start_matches('s').parse(),
            )?))
        }
        // absolute
        else {
            Ok(Pos::Absolute(err_to_string(s.parse())?))
        }
    }
}
fn err_to_string<T, E: ToString>(item: Result<T, E>) -> Result<T, String> {
    item.map_err(|error| error.to_string())
}
#[derive(Clone, Copy)]
struct SmartVector {
    x: Pos,
    y: Pos,
}
impl SmartVector {
    fn new(x: Option<&str>, y: Option<&str>, allow_none: bool) -> Result<SmartVector, String> {
        match x {
            Some(x) => {
                if y.is_none() {
                    return Err("Missing arguments".to_string());
                }
                Ok(SmartVector {
                    x: x.parse()?,
                    y: y.unwrap().parse()?,
                })
            }
            None => match allow_none {
                true => Ok(SmartVector {
                    x: Pos::RelativeSelector(0),
                    y: Pos::RelativeSelector(0),
                }),
                false => Err("Missing arguments".to_string()),
            },
        }
    }
    fn to_absolute(self, player: &Player) -> Vector {
        Vector {
            x: self.x.to_absolute(player.pos.x, player.selector.x),
            y: self.y.to_absolute(player.pos.y, player.selector.y),
        }
    }
}
fn arg(src: Option<&str>) -> Result<&str, String> {
    match src {
        Some(arg) => Ok(arg),
        None => Err("Missing argument".to_string()),
    }
}
