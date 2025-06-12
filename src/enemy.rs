use crate::{Vector, Player, board::BackTrace};
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector,
    pub active: bool,
    pub reachable: bool
}
impl Enemy {
    pub fn new(pos: Vector, variant: Variant) -> Enemy {
        Enemy {
            health: 3,
            variant,
            stun: 0,
            windup: 0,
            pos,
            active: false,
            reachable: false
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic => '1'
            },
            Some({
                let mut out = crate::Style::new();
                if self.active { out.yellow(); }
                if self.stun > 0 { out.background_blue(); }
                else if self.windup > 0 { out.background_red().intense_background(true); }
                out
            })
        )
    }
    pub fn is_stunned(&self) -> bool {
        self.stun > 0
    }
    pub fn is_windup(&self) -> bool {
        self.windup > 0
    }
    pub fn apply_dashstun(&mut self) {
        self.stun += self.variant.dash_stun();
    }
    // returns whether or not it was killed
    pub fn attacked(&mut self) -> bool {
        self.health -= 1;
        self.active = true;
        self.health == 0
    }
    pub fn think(&mut self, board_size: Vector, backtraces: &Vec<BackTrace>, player: &mut Player) {
        if !self.active {
            match backtraces[board_size.x*self.pos.y + self.pos.x].cost {
                Some(cost) => {
                    if cost > (crate::random() & 0b0000_0111) as usize { return }
                    self.active = true;
                }
                None => return
            }
        }
        if self.stun != 0 {
            self.stun -= 1;
            return
        }
        if player.pos.x.abs_diff(self.pos.x) < 2 && player.pos.y.abs_diff(self.pos.y) < 2 {
            if self.windup == 0 {
                self.windup = self.variant.windup();
                return
            }
            self.windup -= 1;
            if self.windup == 0 {
                if let Err(_) = player.attacked((crate::random() & 0b0000_0011) as usize + 3) {
                    self.stun = self.variant.parry_stun();
                }
            }
        }
        else {
            self.windup = 0;
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Variant {
    Basic
}
impl Variant {
    fn windup(self) -> usize {
        match self {
            Variant::Basic => 1
        }
    }
    fn parry_stun(self) -> usize {
        match self {
            Variant::Basic => 3
        }
    }
    fn dash_stun(self) -> usize {
        match self {
            Variant::Basic => 1
        }
    }
    // returns kill reward in energy, then health
    // per energy
    pub fn kill_value(self) -> (usize, usize) {
        match self {
            Variant::Basic => (1, 5)
        }
    }
}
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Basic => write!(f, "basic")
        }
    }
}
impl std::str::FromStr for Variant {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Variant::Basic),
            _ => Err("invalid variant".to_string())
        }
    }
}
