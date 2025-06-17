use crate::Style;
pub struct Spell;
impl Spell {
    pub const SYMBOL: char = '∆';
    pub const STYLE: Style = *Style::new().purple().intense(true);
}
