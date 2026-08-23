use crate::board::EnemyID;
use crate::board::MapType;
use crate::board::Tile;
use crate::board::boon::BoonID;
use crate::math::*;
use crate::state::State;
use crate::upgrade::UpgradeID;
use crate::upgrade::Upgrades;
use abes_nice_things::Style;
use abes_nice_things::{FromBinary, ToBinary};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WalkTrigger {
    Exit,
    Upgrade(UpgradeID),
    /// The boon id and whether or not it has been used
    Boon(BoonID, bool),
}
impl ToBinary for WalkTrigger {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> anyhow::Result<()> {
        match self {
            Self::Exit => 0_u8.to_binary(binary),
            Self::Upgrade(upgrade) => {
                1_u8.to_binary(binary)?;
                upgrade.to_binary(binary)
            }
            Self::Boon(boon, used) => {
                2_u8.to_binary(binary)?;
                boon.to_binary(binary)?;
                used.to_binary(binary)
            }
        }
    }
}
impl FromBinary for WalkTrigger {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> anyhow::Result<Self> {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Exit,
            1 => Self::Upgrade(UpgradeID::from_binary(binary)?),
            2 => Self::Boon(BoonID::from_binary(binary)?, bool::from_binary(binary)?),
            invalid => {
                anyhow::bail!("Attempted to load WalkTrigger with invalid discriminant {invalid}")
            }
        })
    }
}
impl WalkTrigger {
    pub fn render(&self, state: &State) -> (char, Option<Style>) {
        match self {
            Self::Exit => ('∏', None),
            Self::Upgrade(id) => (
                'U',
                Some(match state.player.money >= id.to_upgrade().cost {
                    true => *Style::new().green(),
                    false => *Style::new().red(),
                }),
            ),
            // unused
            Self::Boon(boon, false) => boon.get_boon().unused_render,
            Self::Boon(boon, true) => boon.get_boon().used_render,
        }
    }
    pub fn get_waila(&self, state: &State) -> Vec<String> {
        match self {
            Self::Exit => vec!["An exit".to_string()],
            Self::Upgrade(id) => {
                // Show name, price, then what stack it would be out of max, then the description
                let upgrade = id.to_upgrade();
                vec![
                    upgrade.name.to_string(),
                    format!("cost: {}", upgrade.cost),
                    format!(
                        "{}/{}",
                        state.player.upgrades.num_stacks(*id),
                        upgrade.max_stacks,
                    ),
                    upgrade.description.to_string(),
                ]
            }
            Self::Boon(boon, _) => {
                vec![
                    boon.get_boon().name.to_string(),
                    boon.get_boon().waila.to_string(),
                ]
            }
        }
    }
    /// Handle the case of a player walking on the trigger. It returns if this should be deleted
    /// afterwards
    pub fn handle_player(self, state: &mut State) -> bool {
        match self {
            Self::Exit => {
                match state.board.map_type {
                    // Go to shop
                    MapType::Normal => state.go_to_shop(),
                    // Go to next level
                    MapType::Shop => {
                        state.board = state.next_level.take().unwrap().1.unwrap().unwrap();
                        state.player.position = Vector::new(1, 1);
                        state.player.selector = Vector::new(1, 1);
                    }
                }
                false
            }
            Self::Upgrade(id) => {
                let upgrade = id.to_upgrade();
                // If they are poor then we do nothing, well, we laugh but that's it
                if state.player.money < upgrade.cost {
                    return false;
                }

                if matches!(
                    state
                        .get_input(
                            format!(
                                "Do you want to buy {} for {}?[y/n] ",
                                upgrade.name, upgrade.cost
                            )
                            .as_str(),
                        )
                        .to_lowercase()
                        .as_str(),
                    "y" | "yes"
                        | "yup"
                        | "yuppers"
                        | "sir yes sir"
                        | "yupperino"
                        | "ye"
                        | "yah"
                        | "give me your clothes your boots and your motorcycle"
                        | "yass"
                        | "yes daddy"
                        | "yass queen"
                        | "yass qween"
                        | "shut up and take my money"
                ) {
                    state.player.money -= upgrade.cost;
                    Upgrades::buy(state, id);
                    true
                } else {
                    false
                }
            }
            // Boons do not get deleted
            // unused
            Self::Boon(boon, false) => {
                (boon.get_boon().effect)(state);
                // We know that the player's position is our position
                state.board[state.player.position] =
                    Some(Tile::WalkTrigger(WalkTrigger::Boon(boon, true)));
                false
            }
            // used
            Self::Boon(_, true) => false,
        }
    }
    /// Handle the case of an enemy walking on the trigger, returns if this should be deleted
    /// afterwards
    pub fn handle_enemy(self, _state: &mut State, _enemy: EnemyID) -> bool {
        false
    }
}
