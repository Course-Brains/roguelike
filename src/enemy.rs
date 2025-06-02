use crate::Vector;
#[derive(Clone, Copy)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector
}
impl Enemy {
    pub fn new(pos: Vector) -> Enemy {
        Enemy {
            health: 3,
            variant: Variant::Basic,
            stun: 0,
            windup: 0,
            pos
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic => '1'
            },
            Some({
                let mut out = *crate::Style::new().yellow();
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
        self.health == 0
    }
    pub fn think(&mut self, board_size: crate::Vector, player: &mut crate::Player) {
        if self.stun != 0 {
            self.stun -= 1;
            return
        }
        let mut x_range = self.pos.x as isize-1..=self.pos.x as isize+1;
        let mut y_range = self.pos.y as isize-1..=self.pos.y as isize+1;
        if self.pos.x == 0 {
            let (low, high) = x_range.into_inner();
            x_range = std::ops::RangeInclusive::new(low+1, high)
        }
        if self.pos.x == board_size.x-2 {
            let (low, high) = x_range.into_inner();
            x_range = std::ops::RangeInclusive::new(low, high-1);
        }
        if self.pos.y == 0 {
            let (low, high) = y_range.into_inner();
            y_range = std::ops::RangeInclusive::new(low+1, high)
        }
        if self.pos.y == board_size.y-2 {
            let (low, high) = y_range.into_inner();
            y_range = std::ops::RangeInclusive::new(low, high-1)
        }
        if x_range.contains(&(player.pos.x as isize)) && y_range.contains(&(player.pos.y as isize)) {
            if self.windup == 0 {
                self.windup = self.variant.windup();
                return
            }
            self.windup -= 1;
            if self.windup == 0 {
                if let Err(_) = player.attacked(5) {
                    self.stun = self.variant.parry_stun();
                }
            }
        }
        else {
            self.windup = 0;
        }
    }
}
#[derive(Clone, Copy)]
pub enum Variant {
    Basic
}
impl Variant {
    fn windup(&self) -> usize {
        match self {
            Variant::Basic => 1
        }
    }
    fn parry_stun(&self) -> usize {
        match self {
            Variant::Basic => 3
        }
    }
    fn dash_stun(&self) -> usize {
        match self {
            Variant::Basic => 1
        }
    }
    // returns kill reward in energy, then health
    // per energy
    pub fn kill_value(&self) -> (usize, usize) {
        match self {
            Variant::Basic => (1, 5)
        }
    }
}
