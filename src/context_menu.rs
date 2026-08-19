use crate::board::EnemyID;
use crate::spell::Spell;
use crate::state::Entity;
use crate::state::State;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;
use std::collections::VecDeque;
use std::io::Write;

pub const COLUMNS_NEEDED: usize = 25;

pub struct ContextMenu {
    title: &'static str,
    /// Visual text, what to do when selected, is it active
    pub get_options: fn(&State) -> Vec<(String, Choice, bool)>,
}
impl ContextMenu {
    pub fn get_option_texts(state: &State) -> Vec<String> {
        (state.get_context_menu().get_options)(state)
            .into_iter()
            .map(|(text, _, _)| text)
            .collect()
    }
    pub fn render(state: &mut State, buffer: &mut impl Write) {
        // Act options are purple
        // If we are using the context menu then make everything bold
        let style_base = if state.context_menu_inputs {
            *abes_nice_things::Style::new().bold(true)
        } else {
            abes_nice_things::Style::new()
        };

        // We have the entire screen's height to work with
        let start_column = state.screen_size.x - COLUMNS_NEEDED + 1;
        let context_menu = state.get_context_menu();

        // First we write the title
        write!(
            buffer,
            "\x1b[1;{start_column}H{}{}\x1b[0m",
            style_base.clone().cyan(),
            context_menu.title
        )
        .unwrap();
        // Then we write the separator
        write!(
            buffer,
            "\x1b[2;{start_column}H╶{}╴",
            "─".repeat(COLUMNS_NEEDED - 2)
        )
        .unwrap();

        // -2 for the title
        let available_rows = state.screen_size.y - 2;

        // Then we figure out what range of options we are going to render
        let options = (state.get_context_menu().get_options)(state);
        // Lets make sure we hae a valid option selector position
        let selector = state.get_context_menu_selector_mut();
        if *selector >= options.len() {
            *selector = options.len().saturating_sub(1);
        }
        let width = available_rows.min(options.len());
        let start_index = selector
            .saturating_sub(available_rows / 2)
            .min(options.len().saturating_sub(available_rows / 2));

        // Finally we can actually render them
        // took long enough, jeez
        for (row, index) in (start_index..(start_index + width)).enumerate() {
            let row = row + 3; // 1 because visuals start at 1 and 1 becausse of title
            let mut style = style_base.clone();
            if index == *selector {
                style.background_red().intense(true);
            }
            match options[index].1 {
                Choice::Act(_) => style.green(),
                Choice::Recurse(_, _) => style.yellow(),
                Choice::Info => &mut style,
            };
            if let Choice::Act(_) = options[index].1 {
                style.green();
            }
            if !options[index].2 {
                style.dim(true);
            }

            write!(
                buffer,
                "\x1b[{row};{start_column}H{}{}\x1b[0m",
                style, options[index].0
            )
            .unwrap();
        }
    }
}

pub enum Choice {
    /// The context menu to recurse into and a function to create the argument of it, most of the
    /// time you just want |_| None
    Recurse(usize, Option<Box<dyn Fn(&mut State) -> Argument>>),
    Act(Box<dyn Fn(&mut crate::state::State)>),
    /// Pure info which does nothing when selected
    Info,
}

/// The stack holding the previous and current arguments for when we recurse out as well as the
/// selection index and which context menu it is
///
/// Argument, selection index, context menu
pub type Stack = Vec<(Option<Argument>, usize, ContextMenuID)>;

/// The argument to the context menu itself, this will not get used often
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Argument {
    EnemyID(EnemyID),
    Entity(Entity),
    Spell(Spell, usize),
}
impl ToBinary for Argument {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            Argument::EnemyID(id) => {
                0_u8.to_binary(binary)?;
                id.to_binary(binary)
            }
            Argument::Entity(entity) => {
                1_u8.to_binary(binary)?;
                entity.to_binary(binary)
            }
            Argument::Spell(spell, mana) => {
                2_u8.to_binary(binary)?;
                spell.to_binary(binary)?;
                mana.to_binary(binary)
            }
        }
    }
}
impl FromBinary for Argument {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match u8::from_binary(binary)? {
            0 => Argument::EnemyID(EnemyID::from_binary(binary)?),
            1 => Argument::Entity(Entity::from_binary(binary)?),
            2 => Argument::Spell(Spell::from_binary(binary)?, usize::from_binary(binary)?),
            other => anyhow::bail!("Attempted to make Argument with illegal discriminant {other}"),
        })
    }
}
impl Argument {
    fn enemy_id(self) -> Option<EnemyID> {
        if let Argument::EnemyID(id) = self {
            Some(id)
        } else {
            None
        }
    }
    fn entity(self) -> Option<Entity> {
        if let Argument::Entity(entity) = self {
            Some(entity)
        } else {
            None
        }
    }
    fn spell(self) -> Option<(Spell, usize)> {
        if let Argument::Spell(spell, mana) = self {
            Some((spell, mana))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextMenuID(usize);
impl ContextMenuID {
    pub fn get_context_menu(self) -> &'static ContextMenu {
        &CONTEXT_MENUS[self.0]
    }
    pub fn new(inner: usize) -> ContextMenuID {
        if inner >= CONTEXT_MENUS.len() {
            panic!("Attempted to make invalid context menu id: {inner}")
        }
        ContextMenuID(inner)
    }
}
impl Default for ContextMenuID {
    fn default() -> Self {
        ContextMenuID(MAIN_MENU)
    }
}
impl ToBinary for ContextMenuID {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        self.0.to_binary(binary)
    }
}
impl FromBinary for ContextMenuID {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        let inner = usize::from_binary(binary)?;
        if inner >= CONTEXT_MENUS.len() {
            return Err(anyhow::Error::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not load ContextMenuID from binary due to invalid inner value",
            )));
        }
        Ok(ContextMenuID(inner))
    }
}

const MAIN_MENU: usize = 0;
const DEBUG_MAIN: usize = 1;
const SPECIFIC_ENEMY_DEBUG: usize = 2;
const CHEAT_MAIN: usize = 3;
const EFFECT_SETTER: usize = 4;
const SETTINGS: usize = 5;
const SPELL_MAIN: usize = 6;
const SPECIFIC_SPELL: usize = 7;
const CHEAT_SPELL_LEARN: usize = 8;

static CONTEXT_MENUS: &[ContextMenu] = &[
    // 0: Main menu
    // no argument
    ContextMenu {
        title: "MAIN MENU:",
        get_options: |_| {
            vec![
                (
                    "Spells".to_string(),
                    Choice::Recurse(SPELL_MAIN, None),
                    true,
                ),
                (
                    "Settings".to_string(),
                    Choice::Recurse(SETTINGS, None),
                    true,
                ),
                ("Debug".to_string(), Choice::Recurse(DEBUG_MAIN, None), true),
                (
                    "Cheats".to_string(),
                    Choice::Recurse(CHEAT_MAIN, None),
                    true,
                ),
            ]
        },
    },
    // 1: main debug menu
    // no argument
    ContextMenu {
        title: "DEBUG:",
        get_options: |state| {
            vec![
                (
                    "Specific enemy debug".to_string(),
                    Choice::Recurse(
                        SPECIFIC_ENEMY_DEBUG,
                        Some(Box::new(|state| {
                            Argument::EnemyID(
                                state
                                    .board
                                    .get_enemy_at_position(state.player.selector)
                                    .unwrap(),
                            )
                        })),
                    ),
                    state.board.is_enemy_at_position(state.player.selector) && state.cheats,
                ),
                (
                    "Test board binary".to_string(),
                    Choice::Act(Box::new(|state| {
                        let mut buf = VecDeque::new();
                        state.board.to_binary(&mut buf).unwrap();
                        state.board = crate::board::Board::from_binary(&mut buf).unwrap();
                        assert_eq!(buf.len(), 0);
                        state.feedback = "Success".to_string();
                    })),
                    true,
                ),
                (
                    "Test player binary".to_string(),
                    Choice::Act(Box::new(|state| {
                        let mut buf = VecDeque::new();
                        state.player.to_binary(&mut buf).unwrap();
                        state.player = crate::player::Player::from_binary(&mut buf).unwrap();
                        assert_eq!(buf.len(), 0);
                        state.feedback = "Success".to_string();
                    })),
                    true,
                ),
            ]
        },
    },
    // 2: Specific enemy debug
    // argument of EnemyID
    ContextMenu {
        title: "SPECIFIC ENEMY DEBUG",
        get_options: |state| {
            let mut options = vec![(
                "Log debug info".to_string(),
                Choice::Act(Box::new(|state| {
                    let enemy_id = state
                        .get_current_context_menu_argument()
                        .unwrap()
                        .enemy_id()
                        .unwrap();
                    let enemy = &state.board[enemy_id];
                    abes_nice_things::log!("Logging for enemy({enemy_id:?}): {enemy:#?}");
                })),
                true,
            )];
            let enemy_id = state
                .get_current_context_menu_argument()
                .unwrap()
                .enemy_id()
                .unwrap();
            let enemy = state.board[enemy_id].as_ref();
            // Setting up logging
            options.push((
                "Set log file".to_string(),
                Choice::Act(Box::new(|state| {
                    let enemy_id = state
                        .get_current_context_menu_argument()
                        .unwrap()
                        .enemy_id()
                        .unwrap();
                    if state.board[enemy_id].is_some() {
                        let path = state.get_input("What file? ");
                        state.board[enemy_id]
                            .as_mut()
                            .unwrap()
                            .enable_logging(std::fs::File::create(path).unwrap());
                    }
                })),
                !enemy.is_some_and(|enemy| enemy.has_log_file()),
            ));
            // Turning off logging
            options.push((
                "Disable logging".to_string(),
                Choice::Act(Box::new(|state| {
                    let enemy_id = state
                        .get_current_context_menu_argument()
                        .unwrap()
                        .enemy_id()
                        .unwrap();
                    if let Some(enemy) = &mut state.board[enemy_id] {
                        enemy.disable_logging()
                    }
                })),
                enemy.is_some_and(|enemy| enemy.has_log_file()),
            ));

            // Turning on and off general logging
            options.push((
                format!(
                    "General log: {}",
                    enemy
                        .map(|enemy| enemy.flags.should_general_log().to_string())
                        .unwrap_or("n/a".to_string())
                ),
                Choice::Act(Box::new(|state| {
                    let enemy_id = state
                        .get_current_context_menu_argument()
                        .unwrap()
                        .enemy_id()
                        .unwrap();
                    if let Some(enemy) = &mut state.board[enemy_id] {
                        enemy
                            .flags
                            .set_general_logging(!enemy.flags.should_general_log())
                    };
                })),
                enemy.is_some_and(|enemy| enemy.has_log_file()),
            ));

            // Turning on and off inter room pathfind logging
            options.push((
                format!(
                    "Inter path log: {}",
                    enemy
                        .map(|enemy| enemy.flags.should_inter_pathfind_log().to_string())
                        .unwrap_or("n/a".to_string())
                ),
                Choice::Act(Box::new(|state| {
                    let id = state
                        .get_current_context_menu_argument()
                        .unwrap()
                        .enemy_id()
                        .unwrap();
                    if let Some(enemy) = &mut state.board[id] {
                        enemy.flags.swap_inter_pathfind_log()
                    }
                })),
                enemy.is_some_and(|enemy| enemy.has_log_file()),
            ));

            // Editing effects
            options.push((
                "Set effects".to_string(),
                Choice::Recurse(
                    EFFECT_SETTER,
                    Some(Box::new(move |_| Argument::Entity(Entity::Enemy(enemy_id)))),
                ),
                enemy.is_some() && state.cheats,
            ));

            options
        },
    },
    // 3: main cheat menu
    // no argument
    ContextMenu {
        title: "CHEATS:",
        get_options: |state| {
            let cheats = state.cheats;
            let alive = state.player.is_alive();
            vec![
                (
                    "Enable cheats".to_string(),
                    Choice::Act(Box::new(|state| state.cheats = true)),
                    !cheats,
                ),
                (
                    "Set effects".to_string(),
                    Choice::Recurse(
                        EFFECT_SETTER,
                        Some(Box::new(|_| Argument::Entity(Entity::Player))),
                    ),
                    cheats && alive,
                ),
                (
                    format!(
                        "No interact limit: {}",
                        state.player.flags.no_interact_range_limit()
                    ),
                    Choice::Act(Box::new(|state| {
                        state.player.flags.swap_no_interact_range_limit()
                    })),
                    cheats && alive,
                ),
                (
                    "Open all doors".to_string(),
                    Choice::Act(Box::new(|state| {
                        state.board.open_all_doors();
                    })),
                    cheats,
                ),
                (
                    "Wake all enemies".to_string(),
                    Choice::Act(Box::new(|state| state.board.wake_all_enemies())),
                    cheats,
                ),
                (
                    "Wake specific enemy".to_string(),
                    Choice::Act(Box::new(|state| {
                        let id = state
                            .board
                            .get_enemy_at_position(state.player.selector)
                            .unwrap();
                        state.board[id].as_mut().unwrap().flags.wake();
                    })),
                    state.board.is_enemy_at_position(state.player.selector) && cheats,
                ),
                (
                    "Teleport to selector".to_string(),
                    Choice::Act(Box::new(|state| {
                        state.player.position = state.player.selector
                    })),
                    cheats && alive,
                ),
                (
                    "Save".to_string(),
                    Choice::Act(Box::new(|state| {
                        let path = state.get_input("What file? ");
                        let mut file = std::fs::File::create(path).unwrap();
                        state.to_binary(&mut file).unwrap();
                    })),
                    cheats || !alive,
                ),
                (
                    "Load".to_string(),
                    Choice::Act(Box::new(|state| {
                        let path = state.get_input("What file? ");
                        let mut file = std::fs::File::open(path).unwrap();
                        *state = State::from_binary(&mut file).unwrap();
                    })),
                    true,
                ),
                (
                    "Go to shop".to_string(),
                    Choice::Act(Box::new(|state| state.go_to_shop())),
                    cheats,
                ),
                (
                    "Set energy".to_string(),
                    Choice::Act(Box::new(|state| {
                        use abes_nice_things::Number;
                        if let Some(new_energy) = state
                            .get_input_with_mapper("What do you want to set energy to? ", |input| {
                                input.parse().ok()
                            })
                        {
                            state.player.energy = new_energy;
                            state.player.max_energy.max_assign(new_energy);
                        }
                    })),
                    cheats,
                ),
                (
                    "Set max energy".to_string(),
                    Choice::Act(Box::new(|state| {
                        if let Some(new_max) = state.get_input_with_mapper(
                            "What do you want the new max energy to be? ",
                            |input| input.parse().ok(),
                        ) {
                            state.player.max_energy = new_max
                        }
                    })),
                    cheats,
                ),
                (
                    "Learn spells".to_string(),
                    Choice::Recurse(CHEAT_SPELL_LEARN, None),
                    true,
                ),
            ]
        },
    },
    // 4: Effect setting
    // argument of Entity for which entity's effects
    ContextMenu {
        title: "EFFECT SETTER",
        get_options: |state| {
            let mut options = Vec::new();
            let entity = state
                .get_current_context_menu_argument()
                .unwrap()
                .entity()
                .unwrap();
            // Can't set the effects of the dead
            if match entity {
                Entity::Player => state.player.is_dead(),
                Entity::Enemy(id) => state.board[id].is_none(),
            } {
                return Vec::new();
            }
            let effect_tracker = match entity {
                Entity::Player => &state.player.effect_tracker,
                Entity::Enemy(id) => &state.board[id].as_ref().unwrap().effects,
            };
            for effect in 0..crate::effect::EFFECTS.len() {
                let effect = crate::effect::EffectID::from_raw(effect as u8);
                let current = effect_tracker.get(effect);
                let time = match current {
                    Some(time) => time.to_string(),
                    None => "inf".to_string(),
                };
                options.push((
                    format!("{}: {time}", effect.get().name),
                    Choice::Act(Box::new(move |state| {
                        crate::effect::EffectTracker::prompt_set_time(state, effect, entity)
                    })),
                    true,
                ));
            }
            options
        },
    },
    // 5: settings
    // no argument
    ContextMenu {
        title: "SETTINGS",
        get_options: |state| {
            let mut out = Vec::new();
            for (index, (name, value)) in state
                .unlocked_settings
                .get_names_and_values()
                .into_iter()
                .enumerate()
            {
                let row = index + 3;
                let col = state.screen_size.x - COLUMNS_NEEDED + name.len() + 3;
                out.push((
                    format!("{name}: {value}"),
                    Choice::Act(Box::new(move |state| {
                        print!("\x1b[{row};{col}H\x1b[0K"); // Position cursor and clear old data
                        std::io::stdout().flush().unwrap();
                        state.unlocked_settings.prompt_set_value(name);
                        crate::settings::save_to_file(
                            &state.unlocked_settings,
                            state.locked_settings(),
                        );
                    })),
                    true,
                ));
            }
            out
        },
    },
    // 6: spell main
    // no argument
    ContextMenu {
        title: "Spells",
        get_options: |state| {
            let mut options = Vec::new();

            for spell in state.player.get_known_spells().iter().cloned() {
                options.push((
                    format!("{}: {}", spell.minimum_mana(), spell.get_name()),
                    Choice::Recurse(
                        SPECIFIC_SPELL,
                        Some(Box::new(move |_| {
                            Argument::Spell(spell.clone(), spell.minimum_mana())
                        })),
                    ),
                    true,
                ));
            }

            options
        },
    },
    // 7: specific spell
    // argument of (Spell, usize)
    // being the spell to cast and the currently selected amount of mana to use
    ContextMenu {
        title: "Cast",
        get_options: |state| {
            let (spell, mana) = state
                .get_current_context_menu_argument()
                .unwrap()
                .spell()
                .unwrap();
            let mut options = vec![
                (format!("Spell: {}", spell.get_name()), Choice::Info, true),
                (
                    format!("Type: {}", spell.spell_type_name()),
                    Choice::Info,
                    true,
                ),
                (
                    "cast".to_string(),
                    Choice::Act(Box::new(move |state| {
                        state.player.energy -= mana;
                        match spell {
                            Spell::Position(spell) => spell.cast(
                                state,
                                state.player.position,
                                Entity::Player,
                                state.player.selector,
                                mana,
                            ),
                            Spell::Contact(contact) => contact.cast(
                                state,
                                Entity::Player,
                                Entity::Enemy(
                                    state
                                        .board
                                        .get_enemy_at_position(state.player.selector)
                                        .unwrap(),
                                ),
                            ),
                        }
                        state.increment();
                    })),
                    state.player.energy >= mana
                        && (!spell.is_contact()
                            || (state.player.within_interact_range(state.player.selector)
                                && state.board.is_enemy_at_position(state.player.selector))),
                ),
                (
                    format!("mana: {mana}"),
                    Choice::Act(Box::new(|state| {
                        let spell = state
                            .get_current_context_menu_argument()
                            .unwrap()
                            .spell()
                            .unwrap()
                            .0;
                        loop {
                            let input = state.get_input("How much mana do you want to spend? ");
                            if matches!(
                                input.as_str(),
                                "cancel" | "c" | "stop" | "s" | "back" | "b"
                            ) {
                                return;
                            }
                            match input.parse() {
                                Ok(new_mana) => {
                                    if new_mana < spell.minimum_mana() {
                                        state.feedback = format!(
                                            "You must use at least {} mana",
                                            spell.minimum_mana()
                                        );
                                    } else {
                                        state.set_current_context_menu_argument(Argument::Spell(
                                            spell, new_mana,
                                        ));
                                        break;
                                    }
                                }
                                Err(error) => state.feedback = error.to_string(),
                            }
                            crate::bell(None).unwrap();
                            state.render();
                        }
                    })),
                    true,
                ),
            ];

            for info in spell.get_info(mana) {
                options.push((info, Choice::Info, true));
            }

            options
        },
    },
    // 8: spell learn cheat
    // no argument
    ContextMenu {
        title: "Learn spell cheats",
        get_options: |state| {
            let mut options = Vec::new();
            let cheats = state.cheats;

            for spell in state.player.get_unknown_spells().iter() {
                let spell = *spell;
                options.push((
                    spell.get_name().to_string(),
                    Choice::Act(Box::new(move |state| state.player.learn_spell(spell))),
                    cheats,
                ));
            }

            options
        },
    },
];
