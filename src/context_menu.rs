use crate::state::State;
use std::any::Any;
use std::io::Write;

pub const COLUMNS_NEEDED: usize = 25;

pub struct ContextMenu {
    title: &'static str,
    parent: Option<usize>,
    pub get_options: fn(&State) -> Vec<(String, Choice)>,
}
impl ContextMenu {
    pub fn get_option_texts(state: &State) -> Vec<String> {
        (state.get_context_menu().get_options)(state)
            .into_iter()
            .map(|(text, _)| text)
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
            "\x1b[0;{start_column}H{}{}\x1b[0m",
            style_base.clone().yellow(),
            context_menu.title
        )
        .unwrap();

        // -1 for the title
        let available_rows = state.screen_size.y - 1;

        // Then we figure out what range of options we are going to render
        let options = (state.get_context_menu().get_options)(state);
        // Lets make sure we hae a valid option selector position
        if state.context_menu_selector >= options.len() {
            state.context_menu_selector = 0;
        }
        let width = available_rows.min(options.len());
        let start_index = state
            .context_menu_selector
            .saturating_sub(available_rows / 2)
            .min(options.len().saturating_sub(available_rows / 2));

        // Finally we can actually render them
        // took long enough, jeez
        for (row, index) in (start_index..(start_index + width)).enumerate() {
            let row = row + 2; // 1 because visuals start at 1 and 1 becausse of title
            let mut style = style_base.clone();
            if index == state.context_menu_selector {
                style.background_red().intense_background(true);
            }
            if let Choice::Act(_) = options[index].1 {
                style.purple();
            }

            write!(
                buffer,
                "\x1b[{row};{start_column}H{}{}\x1b[0m",
                style, options[index].0
            )
            .unwrap();
        }
    }
    pub fn get_parent(&self) -> Option<ContextMenuID> {
        self.parent.map(|index| ContextMenuID(index))
    }
}

pub enum Choice {
    /// The context menu to recurse into and a function to create the argument of it, most of the
    /// time you just want |_| None
    Recurse(usize, fn(&mut State) -> Argument),
    Act(fn(&mut crate::state::State)),
}

/// The argument to the context menu itself, this will not get used often
pub type Argument = Option<Box<dyn Any>>;
/// The stack holding the previous and current arguments for when we recurse out
pub type Stack = Vec<Argument>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContextMenuID(usize);
impl ContextMenuID {
    pub fn get_context_menu(self) -> &'static ContextMenu {
        &CONTEXT_MENUS[self.0]
    }
    /// Please don't use this if you aren't certain, I don't want to have to find that bug
    pub fn new_unchecked(inner: usize) -> ContextMenuID {
        ContextMenuID(inner)
    }
}
impl Default for ContextMenuID {
    fn default() -> Self {
        ContextMenuID(MAIN_MENU)
    }
}

const MAIN_MENU: usize = 0;
const DEBUG_MAIN: usize = 1;
const CHEAT_MAIN: usize = 2;

static CONTEXT_MENUS: &[ContextMenu] = &[
    // 0: Main menu
    // no argument
    ContextMenu {
        title: "MAIN MENU:",
        parent: None,
        get_options: |_| {
            vec![
                ("Debug".to_string(), Choice::Recurse(DEBUG_MAIN, |_| None)),
                ("Cheats".to_string(), Choice::Recurse(CHEAT_MAIN, |_| None)),
            ]
        },
    },
    // 1: main debug menu
    // no argument
    ContextMenu {
        title: "DEBUG:",
        parent: Some(MAIN_MENU),
        get_options: |_| Vec::new(),
    },
    // 2: main cheat menu
    // no argument
    ContextMenu {
        title: "CHEATS:",
        parent: Some(MAIN_MENU),
        get_options: |_| Vec::new(),
    },
];
