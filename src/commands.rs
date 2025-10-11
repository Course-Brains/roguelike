use crate::{
    Entity, ItemType, Player, Spell, State, Style, Vector, spell::SpellCircle,
    upgrades::UpgradeType,
};
use abes_nice_things::{FromBinary, Split, ToBinary};
use std::net::TcpListener;
use std::sync::mpsc::Sender;
// Due to the input system, no commands can use stdin
pub enum Command {
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
    SetPiece(SmartVector, String),
    LoadNext,
    LoadShop,
    Effect(String),
    Give(ItemType, Option<usize>),
    SetMoney(usize),
    Upgrade(UpgradeType),
    SetDetectMod(isize),
    SetPerception(usize),
    Cast(Spell),
    CreateCircle(Spell, usize, SmartVector, SmartVector),
    GetData(SmartVector),
    GetBoss,
    CountEnemies,
    Checksum,
    SetBench(bool),
    EnableLog(usize, String),
    ListReachableEnemies,
    NavStepthrough(bool, Option<usize>),
    ShowLineOfSight(bool, Option<usize>),
    SetLimb(String, String),
    SetFeedback(String),
    ToggleShowReachable,
    Cheats,
    KillPlayer,
    GetFeedback,
    DesignateBoss(usize),
    ShowNav(bool, Option<usize>),
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
            "set_piece" | "piece" => Ok(Command::SetPiece(
                SmartVector::new(iter.next(), iter.next(), false)?,
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
                parse(iter.next())?,
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
            "list_reachable_enemies" => Ok(Command::ListReachableEnemies),
            "nav_stepthrough" => Ok(Command::NavStepthrough(
                parse(iter.next())?,
                match iter.next() {
                    Some(s) => Some(parse(Some(s))?),
                    None => None,
                },
            )),
            "show_line_of_sight" => Ok(Command::ShowLineOfSight(
                parse(iter.next())?,
                match iter.next() {
                    Some(s) => Some(parse(Some(s))?),
                    None => None,
                },
            )),
            "set_limb" | "limb" => Ok(Command::SetLimb(parse(iter.next())?, parse(iter.next())?)),
            "set_feedback" => Ok(Command::SetFeedback(
                iter.map(|s| s.to_string() + " ").collect(),
            )),
            "toggle_show_reachable" => Ok(Command::ToggleShowReachable),
            "cheats" => Ok(Command::Cheats),
            "kill_player" => Ok(Command::KillPlayer),
            "get_feedback" => Ok(Command::GetFeedback),
            "designate_boss" => Ok(Command::DesignateBoss(parse(iter.next())?)),
            "show_nav" => Ok(Command::ShowNav(
                parse(iter.next())?,
                match iter.next() {
                    Some(s) => Some(parse(Some(s))?),
                    None => None,
                },
            )),
            _ => Err(format!("Unknown command: ({string})")),
        }
    }
    pub fn enact(self, state: &mut State, out: &mut Sender<String>) {
        if self.is_cheat() && !crate::CHEATS.load(crate::RELAXED) {
            out.send(
                "Attempted to use a command that requires cheats without cheats enabled,\
                please turn on cheats."
                    .to_string(),
            )
            .unwrap();
            return;
        }
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
                let pos = pos.to_absolute(&state.player);
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
            Command::CreateCircle(spell, energy, pos, aim) => {
                state.board.spells.push(SpellCircle::new_player(
                    spell,
                    pos.to_absolute(&state.player),
                    Some(aim.to_absolute(&state.player)),
                    energy,
                ))
            }
            Command::GetData(pos) => {
                let pos = pos.to_absolute(&state.player);
                let index = state.board.to_index(pos);
                out.send(
                    if state.board.seen[index] {
                        "It has been seen"
                    } else {
                        "It has NOT been seen"
                    }
                    .to_string(),
                )
                .unwrap();
                out.send(
                    if state.board.visible[index] {
                        "It is within the visibility flood"
                    } else {
                        "It is NOT within the visibility flood"
                    }
                    .to_string(),
                )
                .unwrap();
                out.send(
                    if state.is_visible(pos) {
                        "It is visible"
                    } else {
                        "It is NOT visible"
                    }
                    .to_string(),
                )
                .unwrap();
                out.send(
                    if state.board.reachable[index] {
                        "It is reachable"
                    } else {
                        "It is NOT reachable"
                    }
                    .to_string(),
                )
                .unwrap();
                out.send(if let Some(piece) = &state.board[pos] {
                    format!("It is a {piece}")
                } else {
                    "It does NOT contain a piece".to_string()
                })
                .unwrap();
                out.send(
                    if state.player.pos == pos {
                        "It contains the player"
                    } else {
                        "It does NOT contain the player"
                    }
                    .to_string(),
                )
                .unwrap();

                let mut has_enemy = false;
                for (index, enemy) in state.board.enemies.iter().enumerate() {
                    if enemy.try_read().unwrap().pos == pos {
                        out.send(format!(
                            "It contains enemy {}, which is a {}",
                            index,
                            enemy.try_read().unwrap().variant
                        ))
                        .unwrap();
                        has_enemy = true;
                    }
                }
                if !has_enemy {
                    out.send("It does NOT contain an enemy".to_string())
                        .unwrap()
                }
                let mut has_special = false;
                for special in state.board.specials.iter() {
                    if let Some(special) = special.upgrade()
                        && special.pos == pos
                    {
                        out.send(format!(
                            "It has a special with visuals: {}{}\x1b0m",
                            special.style.unwrap_or(Style::new()),
                            special.ch
                        ))
                        .unwrap();
                        has_special = true;
                    }
                }
                if !has_special {
                    out.send("It does NOT contain a special".to_string())
                        .unwrap()
                }
                let mut has_spell = false;
                for circle in state.board.spells.iter() {
                    if circle.pos == pos {
                        out.send(format!("It contains the spell circle: ({})", circle.spell))
                            .unwrap();
                        match &circle.caster {
                            Some(arc) => {
                                for (index, enemy) in state.board.enemies.iter().enumerate() {
                                    if std::sync::Arc::ptr_eq(arc, enemy) {
                                        out.send(format!("  And was cast by enemy {index}"))
                                            .unwrap();
                                    }
                                }
                            }
                            None => out
                                .send("  And was cast by the player".to_string())
                                .unwrap(),
                        }
                        has_spell = true;
                    }
                }
                if !has_spell {
                    out.send("It does NOT contain a spell circle".to_string())
                        .unwrap();
                }
            }
            Command::GetBoss => {
                for boss in state.board.bosses.iter() {
                    if let Some(arc) = boss.sibling.upgrade() {
                        out.send(format!(
                            "There is a {} at {}",
                            arc.try_read().unwrap().variant,
                            arc.try_read().unwrap().pos
                        ))
                        .unwrap();
                    } else {
                        out.send(format!("There is a gate at {}", boss.last_pos))
                            .unwrap();
                    }
                }
                if state.board.bosses.is_empty() {
                    out.send("There are no bosses".to_string()).unwrap();
                }
            }
            Command::CountEnemies => {
                let mut count = std::collections::HashMap::new();
                for enemy in state.board.enemies.iter() {
                    let variant_num = enemy.try_read().unwrap().variant.to_key();
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
                out.send(format!("There are {} archer", count.get(&6).unwrap_or(&0)))
                    .unwrap();
                out.send(format!(
                    "There are {} archer_boss",
                    count.get(&7).unwrap_or(&0)
                ))
                .unwrap();
            }
            Command::Checksum => {
                out.send(match crate::generator::checksum(&state.board) {
                    Ok(_) => "Checksum passed".to_string(),
                    Err(error) => error,
                })
                .unwrap();
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
            Command::ListReachableEnemies => {
                for (index, enemy) in state.board.enemies.iter().enumerate() {
                    let pos = enemy.try_read().unwrap().pos;
                    if state.board.is_reachable(pos) {
                        out.send(format!("{index} at {pos}")).unwrap();
                    }
                }
            }
            Command::NavStepthrough(new_state, index) => {
                state.nav_stepthrough = new_state;
                state.nav_stepthrough_index = index;
            }
            Command::ShowLineOfSight(new_state, index) => match index {
                Some(index) => {
                    state.board.enemies[index]
                        .try_write()
                        .unwrap()
                        .show_line_of_sight = new_state;
                }
                None => {
                    for enemy in state.board.enemies.iter() {
                        enemy.try_write().unwrap().show_line_of_sight = new_state;
                    }
                }
            },
            Command::SetLimb(slot, choice) => {
                if let Err(error) = state.player.limbs.set(slot, choice) {
                    out.send(error).unwrap();
                }
            }
            Command::SetFeedback(feedback) => {
                *crate::feedback() = feedback;
            }
            Command::ToggleShowReachable => {
                crate::SHOW_REACHABLE.fetch_xor(true, crate::RELAXED);
            }
            Command::Cheats => {
                crate::CHEATS.store(true, crate::RELAXED);
            }
            Command::KillPlayer => {
                state.player.killer = Some(("falling out of the world", None, 0));
            }
            Command::GetFeedback => {
                out.send(format!("Current feedback is: \"{}\"", *crate::feedback()))
                    .unwrap();
            }
            Command::DesignateBoss(index) => {
                let arc = state.board.enemies[index].clone();
                state.board.bosses.push(crate::board::Boss {
                    last_pos: arc.try_read().unwrap().pos,
                    sibling: std::sync::Arc::downgrade(&arc),
                });
            }
            Command::ShowNav(new_state, index) => {
                state.show_nav = new_state;
                state.show_nav_index = index;
            }
        }
    }
    fn is_cheat(&self) -> bool {
        // All aside from listed require cheats
        !matches!(
            self,
            Self::Redraw
                | Self::ForceFlood
                | Self::Checksum
                | Self::SetBench(_)
                | Self::SetFeedback(_)
                | Self::ToggleShowReachable
                | Self::Cheats
                | Self::GetFeedback
        )
    }
}
pub fn listen(out: Sender<crate::CommandInput>) -> Sender<String> {
    let (console_tx, console_rx) = std::sync::mpsc::channel();
    let tx_clone = console_tx.clone();
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
                    Ok(command) => out.send(crate::CommandInput::Command(command)).unwrap(),
                    Err(error) => tx_clone.send(error).unwrap(),
                }
            }
        });
        std::thread::spawn(move || {
            loop {
                console_rx.recv().unwrap().to_binary(&mut write).unwrap();
            }
        });
    });
    console_tx
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
pub struct SmartVector {
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
