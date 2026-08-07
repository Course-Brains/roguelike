use crate::board::EnemyID;
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
            style_base.clone().yellow(),
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
    Recurse(usize, fn(&mut State) -> Option<Argument>),
    Act(Box<dyn Fn(&mut crate::state::State)>),
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
}
impl ToBinary for Argument {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<()> {
        match self {
            Argument::EnemyID(id) => {
                false.to_binary(binary)?;
                id.to_binary(binary)
            }
            Argument::Entity(entity) => {
                true.to_binary(binary)?;
                entity.to_binary(binary)
            }
        }
    }
}
impl FromBinary for Argument {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(match bool::from_binary(binary)? {
            false => Argument::EnemyID(EnemyID::from_binary(binary)?),
            true => Argument::Entity(Entity::from_binary(binary)?),
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

static CONTEXT_MENUS: &[ContextMenu] = &[
    // 0: Main menu
    // no argument
    ContextMenu {
        title: "MAIN MENU:",
        get_options: |_| {
            vec![
                (
                    "Debug".to_string(),
                    Choice::Recurse(DEBUG_MAIN, |_| None),
                    true,
                ),
                (
                    "Cheats".to_string(),
                    Choice::Recurse(CHEAT_MAIN, |_| None),
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
                    Choice::Recurse(SPECIFIC_ENEMY_DEBUG, |state| {
                        Some(Argument::EnemyID(
                            state
                                .board
                                .get_enemy_at_position(state.player.selector)
                                .unwrap(),
                        ))
                    }),
                    state.board.is_enemy_at_position(state.player.selector),
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
                        let path = state.get_input("What file? ".to_string());
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
            options
        },
    },
    // 3: main cheat menu
    // no argument
    ContextMenu {
        title: "CHEATS:",
        get_options: |state| {
            vec![
                (
                    "Set effects".to_string(),
                    Choice::Recurse(EFFECT_SETTER, |_| Some(Argument::Entity(Entity::Player))),
                    true,
                ),
                (
                    format!(
                        "No interact limit: {}",
                        state.player.no_interact_range_limit
                    ),
                    Choice::Act(Box::new(|state| {
                        state.player.no_interact_range_limit ^= true;
                    })),
                    true,
                ),
                (
                    "Open all doors".to_string(),
                    Choice::Act(Box::new(|state| {
                        state.board.open_all_doors();
                    })),
                    true,
                ),
                (
                    "Wake all enemies".to_string(),
                    Choice::Act(Box::new(|state| state.board.wake_all_enemies())),
                    true,
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
                    state.board.is_enemy_at_position(state.player.selector),
                ),
                (
                    "Teleport to selector".to_string(),
                    Choice::Act(Box::new(|state| {
                        state.player.position = state.player.selector
                    })),
                    true,
                ),
                (
                    "Save".to_string(),
                    Choice::Act(Box::new(|state| {
                        let path = state.get_input("What file?".to_string());
                        let mut file = std::fs::File::create(path).unwrap();
                        state.to_binary(&mut file).unwrap();
                    })),
                    true,
                ),
                (
                    "Load".to_string(),
                    Choice::Act(Box::new(|state| {
                        let path = state.get_input("What file?".to_string());
                        let mut file = std::fs::File::open(path).unwrap();
                        *state = State::from_binary(&mut file).unwrap();
                    })),
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
            for effect in 0..crate::effect::EFFECTS.len() {
                let effect = crate::effect::EffectID::from_raw(effect as u8);
                let current = state.player.effect_tracker.get(effect);
                let time = match current {
                    Some(time) => time.to_string(),
                    None => "inf".to_string(),
                };
                options.push((
                    format!("{}: {time}", effect.get().name),
                    Choice::Act(Box::new(move |state| {
                        crate::effect::EffectTracker::prompt_set_time(
                            state,
                            Entity::Player,
                            effect,
                        );
                    })),
                    true,
                ));
            }
            options
        },
    },
];
