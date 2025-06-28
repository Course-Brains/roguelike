use crate::{Style, pieces::spell::Stepper};
use std::fmt::Display;
const CHAR: char = '?';
const STYLE: Style = Style::new();
#[derive(Clone, Copy, Debug)]
pub struct Item {
    item_type: crate::ItemType,
}
impl Item {
    pub fn render() -> (char, Option<Style>) {
        (CHAR, Some(STYLE))
    }
    pub fn on_step(&self, stepper: Stepper<'_>) -> bool {
        if let Stepper::Player(player) = stepper {
            if player.money > self.item_type.price() {
                return player.add_item(self.item_type);
            }
        }
        false
    }
    pub fn get_desc(&self) -> &'static str {
        self.item_type.get_desc()
    }
}
impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.item_type.fmt(f)
    }
}
