#[derive(Clone, Copy)]
pub struct Enemy {
    pub health: usize,
    variant: Variant,
    stun: usize,
    windup: usize,
}
impl Enemy {
    pub fn new() -> Enemy {
        Enemy {
            health: 3,
            variant: Variant::Basic,
            stun: 0,
            windup: 0
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic => '1'
            },
            Some({
                let mut out = *crate::Style::new().yellow();
                if self.stun > 0 {
                    out.background_blue().intense_background(true);
                }
                else if self.windup > 0 {
                    out.background_red().intense_background(true);
                }
                out
            })
        )
    }
    pub fn apply_dashstun(&mut self) {
        self.stun += self.variant.dash_stun();
    }
    // returns whether or not it was killed
    pub fn attacked(&mut self) -> bool {
        self.health -= 1;
        self.health == 0
    }
    pub fn think(&mut self, pos: crate::Vector, board_size: crate::Vector, player: &mut crate::Player) {
        if self.stun != 0 {
            self.stun -= 1;
            return
        }
        let mut x_range = pos.x as isize-1..=pos.x as isize+1;
        let mut y_range = pos.y as isize-1..=pos.y as isize+1;
        if pos.x == 0 {
            let (low, high) = x_range.into_inner();
            x_range = std::ops::RangeInclusive::new(low+1, high)
        }
        if pos.x == board_size.x-2 {
            let (low, high) = x_range.into_inner();
            x_range = std::ops::RangeInclusive::new(low, high-1);
        }
        if pos.y == 0 {
            let (low, high) = y_range.into_inner();
            y_range = std::ops::RangeInclusive::new(low+1, high)
        }
        if pos.y == board_size.y-2 {
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
                if player.blocking {
                    self.stun = self.variant.parry_stun();
                    return
                }
                player.health -= 5;
            }
        }
        else {
            self.windup = 0;
        }
    }
}
#[derive(Clone, Copy)]
enum Variant {
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
}
