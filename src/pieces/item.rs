use crate::{Random, Style, pieces::spell::Stepper};
use std::fmt::Display;
const CHAR: char = '?';
const STYLE: Style = Style::new();
#[derive(Clone, Copy, Debug)]
pub struct Item {
    item_type: crate::ItemType,
}
impl Item {
    pub fn new(item_type: Option<crate::ItemType>) -> Item {
        Item {
            item_type: item_type.unwrap_or(crate::ItemType::random()),
        }
    }
    pub fn render() -> (char, Option<Style>) {
        (CHAR, Some(STYLE))
    }
    pub fn on_step(&self, stepper: Stepper<'_>) -> bool {
        crate::log!("Item({self}) was stepped on");
        if let Stepper::Player(player) = stepper {
            crate::log!("  Attempting pickup");
            if player.money > self.item_type.price() {
                crate::log!("    Pickup is valid");
                if player.add_item(self.item_type) {
                    player.money -= self.item_type.price();
                    crate::log!("      Picked up item, money is now {}", player.money);
                    return true;
                }
            }
        }
        false
    }
    pub fn get_desc(&self, lock: &mut impl std::io::Write) {
        write!(
            lock,
            "item: {}. price: {}",
            self.item_type.get_desc(),
            self.item_type.price()
        )
        .unwrap();
    }
}
impl Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.item_type.fmt(f)
    }
}
