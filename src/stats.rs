use crate::*;
#[derive(Clone, Debug)]
pub struct Stats {
    // The amount of money when entering each shop
    pub shop_money: Vec<usize>,
    // The total amount of money gained in a run
    pub total_money: usize,
    // how far down you go
    pub depth: usize,
    // How often each item was bought
    pub buy_list: HashMap<ItemType, usize>,
    // What upgrades were held at death
    pub upgrades: Upgrades,
    // How many turns were spent in each completed level
    pub level_turns: Vec<usize>,
    // How many turns were spent in each shop
    pub shop_turns: Vec<usize>,
    // How much damage was taken in total
    pub damage_taken: usize,
    // How much damage was blocked in total
    pub damage_blocked: usize,
    // How much damage was avoided by invulnerability
    pub damage_invulned: usize,
    // How much damage was directly dealt by the player
    pub damage_dealt: usize,
    // How much health was healed
    pub damage_healed: usize,
    // What turn it was when the player died
    pub death_turn: usize,
    // How many of each spell was cast
    pub spell_list: HashMap<Spell, usize>,
    // How many saves were made
    pub num_saves: usize,
    // how many of each enemy type were killed
    pub kills: HashMap<u8, usize>,
    // total energy used
    pub energy_used: usize,
    // reward energy that was lost
    pub energy_wasted: usize,
    // Number of times a door was closed on a boss
    pub cowardice: usize,
    // What enemy type did the killing blow, or none if it was the player
    pub killer: Option<u8>,
    // Number of times a door was opened
    pub doors_opened: usize,
    // Number of times a door was closed
    pub doors_closed: usize,
    // Number of attacks done by you
    pub attacks_done: usize,
    // Number of attacks that dealt damage to you
    pub hits_taken: usize,
    // Number of doors opened with wasd
    pub doors_opened_by_walking: usize,
    // Number of enemies attacked with wasd
    pub enemies_hit_by_walking: usize,
    // The settings used at death
    pub settings: Settings,
    // The number of times the player memorized a position
    pub times_memorized: usize,
    // The number of times the player remembered a position
    pub times_remembered: usize,
    // The damage of the attack that killed the player
    pub killing_damage: usize,
}
impl Stats {
    pub fn new() -> Stats {
        Stats {
            shop_money: Vec::new(),
            total_money: 0,
            depth: 0,
            buy_list: HashMap::new(),
            upgrades: Upgrades::new(),
            level_turns: Vec::new(),
            shop_turns: Vec::new(),
            damage_taken: 0,
            damage_blocked: 0,
            damage_invulned: 0,
            damage_dealt: 0,
            damage_healed: 0,
            death_turn: 0,
            spell_list: HashMap::new(),
            num_saves: 0,
            kills: HashMap::new(),
            energy_used: 0,
            energy_wasted: 0,
            cowardice: 0,
            killer: None,
            doors_opened: 0,
            doors_closed: 0,
            attacks_done: 0,
            hits_taken: 0,
            doors_opened_by_walking: 0,
            enemies_hit_by_walking: 0,
            settings: Settings::default(),
            times_memorized: 0,
            times_remembered: 0,
            killing_damage: 0,
        }
    }
    pub fn collect_death(&mut self, state: &State) {
        self.depth = layer();
        self.upgrades = state.player.upgrades;
        self.death_turn = state.turn;
        let killing_data = state.player.killer.unwrap();
        self.killer = killing_data.1;
        self.killing_damage = killing_data.2;
        self.settings = SETTINGS.clone();
    }
    pub fn add_item(&mut self, item: ItemType) {
        self.buy_list
            .insert(item, self.buy_list.get(&item).unwrap_or(&0) + 1);
    }
    pub fn add_spell(&mut self, spell: Spell) {
        self.spell_list
            .insert(spell, self.spell_list.get(&spell).unwrap_or(&0) + 1);
    }
    pub fn add_kill(&mut self, variant: enemy::Variant) {
        let key = variant.to_key();
        let prev = self.kills.get(&key).unwrap_or(&0);
        self.kills.insert(key, prev + 1);
    }
    pub fn list_kills(&self) {
        for (key, kills) in self.kills.iter() {
            println!("{}: {kills}", enemy::Variant::from_key(*key).kill_name());
        }
        println!();
    }
    pub fn list_killer(&self) {
        println!(
            "{}",
            self.killer
                .map(|key| enemy::Variant::from_key(key).kill_name())
                .unwrap_or("Yourself")
        )
    }
}
impl FromBinary for Stats {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Stats {
            shop_money: Vec::from_binary(binary)?,
            total_money: usize::from_binary(binary)?,
            depth: usize::from_binary(binary)?,
            buy_list: HashMap::from_binary(binary)?,
            upgrades: Upgrades::from_binary(binary)?,
            level_turns: Vec::from_binary(binary)?,
            shop_turns: Vec::from_binary(binary)?,
            damage_taken: usize::from_binary(binary)?,
            damage_blocked: usize::from_binary(binary)?,
            damage_invulned: usize::from_binary(binary)?,
            damage_dealt: usize::from_binary(binary)?,
            damage_healed: usize::from_binary(binary)?,
            death_turn: usize::from_binary(binary)?,
            spell_list: HashMap::from_binary(binary)?,
            num_saves: usize::from_binary(binary)?,
            kills: HashMap::from_binary(binary)?,
            energy_used: usize::from_binary(binary)?,
            energy_wasted: usize::from_binary(binary)?,
            cowardice: usize::from_binary(binary)?,
            killer: Option::from_binary(binary)?,
            doors_opened: usize::from_binary(binary)?,
            doors_closed: usize::from_binary(binary)?,
            attacks_done: usize::from_binary(binary)?,
            hits_taken: usize::from_binary(binary)?,
            doors_opened_by_walking: usize::from_binary(binary)?,
            enemies_hit_by_walking: usize::from_binary(binary)?,
            settings: Settings::from_binary(binary)?,
            times_memorized: usize::from_binary(binary)?,
            times_remembered: usize::from_binary(binary)?,
            killing_damage: usize::from_binary(binary)?,
        })
    }
}
impl ToBinary for Stats {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.shop_money.to_binary(binary)?;
        self.total_money.to_binary(binary)?;
        self.depth.to_binary(binary)?;
        self.buy_list.to_binary(binary)?;
        self.upgrades.to_binary(binary)?;
        self.level_turns.to_binary(binary)?;
        self.shop_turns.to_binary(binary)?;
        self.damage_taken.to_binary(binary)?;
        self.damage_blocked.to_binary(binary)?;
        self.damage_invulned.to_binary(binary)?;
        self.damage_dealt.to_binary(binary)?;
        self.damage_healed.to_binary(binary)?;
        self.death_turn.to_binary(binary)?;
        self.spell_list.to_binary(binary)?;
        self.num_saves.to_binary(binary)?;
        self.kills.to_binary(binary)?;
        self.energy_used.to_binary(binary)?;
        self.energy_wasted.to_binary(binary)?;
        self.cowardice.to_binary(binary)?;
        self.killer.as_ref().to_binary(binary)?;
        self.doors_opened.to_binary(binary)?;
        self.doors_closed.to_binary(binary)?;
        self.attacks_done.to_binary(binary)?;
        self.hits_taken.to_binary(binary)?;
        self.doors_opened_by_walking.to_binary(binary)?;
        self.enemies_hit_by_walking.to_binary(binary)?;
        self.settings.to_binary(binary)?;
        self.times_memorized.to_binary(binary)?;
        self.times_remembered.to_binary(binary)?;
        self.killing_damage.to_binary(binary)
    }
}
pub fn save_stats() {
    if CHEATS.load(RELAXED) {
        return;
    }
    let mut stats_saves: Vec<Stats> = Vec::new();
    if std::fs::exists(STAT_PATH).unwrap() {
        log!("Stats file exists, checking version");
        let mut file = std::fs::File::open(STAT_PATH).unwrap();
        if Version::from_binary(&mut file).unwrap() != SAVE_VERSION {
            log!("!!!Save version mismatch!!!");
            crossterm::queue!(
                std::io::stdout(),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
                Vector::new(0, 0).to_move(),
                crossterm::terminal::EnableLineWrap,
            )
            .unwrap();
            println!(
                "{}The save format in the stats file is different than the current \
                    save format, if you leave the stats file where it is, it will be \
                    deleted, I recommend moving it.\n\x1b[0mPress enter to continue",
                Style::new().red().bold(true).underline(true).intense(true)
            );
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_line(&mut String::new()).unwrap();
        } else {
            stats_saves = Vec::from_binary(&mut file).unwrap();
        }
    }

    stats_saves.push(stats().clone());
    let mut file = std::fs::File::create(STAT_PATH).unwrap();
    SAVE_VERSION.to_binary(&mut file).unwrap();
    stats_saves.to_binary(&mut file).unwrap();
    log!("Saving stats");
}
pub fn view_stats() {
    log!("Entering stats viewer");
    let mut input = String::new();
    let mut file = match std::fs::File::open(STAT_PATH) {
        Ok(file) => file,
        Err(error) => {
            if let std::io::ErrorKind::NotFound = error.kind() {
                println!("There is no stats file, I recommend playing at least once");
                println!("Press enter to go back");
                std::io::stdin().read_line(&mut String::new()).unwrap();
                return;
            }
            panic!("{error}");
        }
    };
    if Version::from_binary(&mut file).unwrap() != SAVE_VERSION {
        println!(
            "{}The save version of the file does not match the \
        current install and therefore cannot be viewed\x1b[0m",
            Style::new().red()
        );
        return;
    }
    log!("Pulling stats from file");
    let stats = Vec::<Stats>::from_binary(&mut file).unwrap();
    let mut index = 0;
    macro_rules! list {
        ($field: ident, $index: ident) => {
            match $index {
                Some(index) => {
                    println!("{index}: {:?}", stats[index].$field);
                }
                None => {
                    for stat in stats.iter() {
                        println!("{:?}", stat.$field);
                    }
                }
            }
        };
        ($field: ident, $index: ident, $method: ident) => {
            match $index {
                Some(index) => {
                    print!("{index}: ");
                    stats[index].$method()
                }
                None => {
                    for stat in stats.iter() {
                        stat.$method()
                    }
                }
            }
        };
    }
    log!("Viewing stats:");
    loop {
        println!("What would you like to do?");
        input.truncate(0);
        std::io::stdin().read_line(&mut input).unwrap();
        let mut split = input.trim().split(' ');
        match split.next().unwrap() {
            "help" => println!("{}", include_str!("stat_help.txt")),
            "next" => {
                if let Ok(offset) = split.next().unwrap_or("1").parse::<usize>() {
                    let new_index = index + offset;
                    if new_index < stats.len() {
                        index = new_index;
                        println!("now at {index}");
                    } else {
                        println!("{new_index} is not a valid index");
                    }
                } else {
                    println!("Expected number, found not number");
                }
            }
            "prev" => {
                if let Ok(offset) = split.next().unwrap_or("1").parse::<usize>() {
                    if offset > index {
                        println!("Attempted to go to negative index");
                    } else {
                        index -= offset;
                        println!("now at {index}");
                    }
                }
            }
            "jump" => {
                if let Some(s) = split.next() {
                    if let Ok(new_index) = s.parse() {
                        if stats.get(new_index).is_some() {
                            index = new_index;
                        } else {
                            println!("{new_index} is not a valid index");
                        }
                    } else {
                        println!("Failed to get index");
                    }
                } else {
                    println!("Expected index to jump to")
                }
            }
            "list" => match split.next() {
                Some(field) => {
                    let index = match split.next() {
                        Some(string) => match string.parse::<usize>() {
                            Ok(index) => Some(index),
                            Err(_) => {
                                eprintln!("Invalid index");
                                continue;
                            }
                        },
                        None => None,
                    };
                    match field {
                        "shop_money" => list!(shop_money, index),
                        "total_money" => list!(total_money, index),
                        "depth" => list!(depth, index),
                        "buy_list" => list!(buy_list, index),
                        "upgrades" => list!(upgrades, index),
                        "level_turns" => list!(level_turns, index),
                        "shop_turns" => list!(shop_turns, index),
                        "damage_taken" => list!(damage_taken, index),
                        "damage_blocked" => list!(damage_blocked, index),
                        "damage_invulned" => list!(damage_invulned, index),
                        "damage_dealt" => list!(damage_dealt, index),
                        "damage_healed" => list!(damage_healed, index),
                        "death_turn" => list!(death_turn, index),
                        "spell_list" => list!(spell_list, index),
                        "num_saves" => list!(num_saves, index),
                        "kills" => list!(kills, index, list_kills),
                        "energy_used" => list!(energy_used, index),
                        "energy_wasted" => list!(energy_wasted, index),
                        "cowardice" => list!(cowardice, index),
                        "killer" => list!(killer, index, list_killer),
                        "doors_opened" => list!(doors_opened, index),
                        "doors_closed" => list!(doors_closed, index),
                        "attacks_done" => list!(attacks_done, index),
                        "hits_taken" => list!(hits_taken, index),
                        "doors_opened_by_walking" => list!(doors_opened_by_walking, index),
                        "enemies_hit_by_walking" => list!(enemies_hit_by_walking, index),
                        "settings" => list!(settings, index),
                        "times_memorized" => list!(times_memorized, index),
                        "times_remembered" => list!(times_remembered, index),
                        other => println!("{other} is not a valid field"),
                    }
                }
                None => println!("{index} out of {}:\n{:#?}", stats.len() - 1, stats[index]),
            },
            "quit" => break,
            other => println!("\"{other}\" is not a valid command"),
        }
    }
}
