use crate::{Board, Player, Vector, board::BackTrace, style::Color};
use std::cell::{RefCell, RefMut};
use std::rc::Rc;
const MAGE_RANGE: usize = 30;
#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub health: usize,
    pub variant: Variant,
    stun: usize,
    windup: usize,
    pub pos: Vector,
    pub active: bool,
    pub reachable: bool,
    pub attacking: bool,
}
impl Enemy {
    pub fn new(pos: Vector, variant: Variant) -> Enemy {
        Enemy {
            health: variant.max_health(),
            variant,
            stun: 0,
            windup: 0,
            pos,
            active: false,
            reachable: false,
            attacking: false,
        }
    }
    pub fn render(&self) -> (char, Option<crate::Style>) {
        (
            match self.variant {
                Variant::Basic => '1',
                Variant::Mage(_) => '2',
            },
            Some({
                let mut out = crate::Style::new();
                if self.active {
                    out.yellow();
                }
                if self.stun > 0 {
                    out.background_blue();
                } else if self.windup > 0 {
                    out.set_background(self.variant.windup_color())
                        .intense_background(true);
                }
                out
            }),
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
    pub fn think(this: Rc<RefCell<Self>>, addr: usize, board: &mut Board, player: &mut Player) {
        let mut this = this.borrow_mut();
        if !this.active {
            if this.variant.detect(&this, board) {
                this.active = true;
            } else {
                return;
            }
        }
        if this.stun != 0 {
            this.stun -= 1;
            return;
        }
        match this.variant {
            Variant::Basic => {
                if player.pos.x.abs_diff(this.pos.x) < 2 && player.pos.y.abs_diff(this.pos.y) < 2 {
                    this.attacking = true;
                    if this.windup == 0 {
                        this.windup = 3;
                        return;
                    }
                    this.windup -= 1;
                    if this.windup == 0 {
                        if let Err(_) =
                            player.attacked((crate::random() & 0b0000_0011) as usize + 3)
                        {
                            this.stun = this.variant.parry_stun();
                        }
                    }
                } else {
                    this.attacking = false;
                    this.windup = 0;
                }
            }
            Variant::Mage(cast_pos) => {
                if this.is_near(player.pos, MAGE_RANGE) {
                    this.attacking = true;
                } else {
                    this.attacking = false;
                    this.windup = 0;
                }
                if this.windup > 1 {
                    this.windup -= 1;
                    return;
                }
                if this.windup == 1 {
                    // cast time BAYBEEE
                    if board[cast_pos].is_none() {
                        board[cast_pos] = Some(crate::board::Piece::Spell);
                    }
                    this.windup = 0;
                }
                match crate::random() & 0b0000_0011 {
                    0 => {
                        // teleport
                        let mut near = Vec::new();
                        for enemy in board.enemies.iter() {
                            if enemy.as_ptr().addr() == addr {
                                continue;
                            }
                            let pos = enemy.borrow().pos;
                            if pos.x.abs_diff(this.pos.x) < MAGE_RANGE
                                && pos.y.abs_diff(this.pos.y) < MAGE_RANGE
                            {
                                near.push(enemy.clone());
                            }
                        }
                        let target = match near.len() > 256 {
                            true => near[crate::random() as usize].clone(),
                            false => near[crate::random() as usize % (near.len() - 1)].clone(),
                        };
                        std::mem::swap(&mut target.borrow_mut().pos, &mut this.pos);
                        crate::RE_FLOOD.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    1 => {
                        // Alert nearby enemies
                        for enemy in board.enemies.iter_mut() {
                            if enemy.as_ptr().addr() == addr {
                                continue;
                            }
                            if enemy.borrow().is_near(this.pos, MAGE_RANGE) {
                                enemy.borrow_mut().active = true;
                            }
                        }
                    }
                    2 => {
                        // spell time
                        if board[player.pos].is_none() {
                            this.windup = 3;
                            this.variant = Variant::Mage(player.pos);
                        }
                    }
                    3 => {
                        // do nothing
                    }
                    _ => unreachable!("Bit and seems to be broken"),
                }
            }
        }
    }
    pub fn is_near(&self, pos: Vector, range: usize) -> bool {
        self.pos.x.abs_diff(pos.x) < range && self.pos.y.abs_diff(pos.y) < range
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Variant {
    Basic,
    // cast position
    Mage(Vector),
}
impl Variant {
    fn detect(self, enemy: &RefMut<Enemy>, board: &Board) -> bool {
        match self {
            Variant::Basic => match board.backtraces[board.x * enemy.pos.y + enemy.pos.x].cost {
                Some(cost) => cost > (crate::random() & 0b0000_0111) as usize,
                None => false,
            },
            Variant::Mage(_) => match board.backtraces[board.x * enemy.pos.y + enemy.pos.x].cost {
                Some(cost) => cost > (((crate::random() & 0b0000_0111) + 1) << 2) as usize,
                None => false,
            },
        }
    }
    fn windup_color(self) -> Color {
        match self {
            Variant::Basic => Color::Red,
            Variant::Mage(_) => Color::Purple,
        }
    }
    fn max_health(self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::Mage(_) => 5,
        }
    }
    fn parry_stun(self) -> usize {
        match self {
            Variant::Basic => 3,
            Variant::Mage(_) => 0,
        }
    }
    fn dash_stun(self) -> usize {
        match self {
            Variant::Basic => 1,
            Variant::Mage(_) => 2,
        }
    }
    // returns kill reward in energy, then health
    // per energy
    pub fn kill_value(self) -> (usize, usize) {
        match self {
            Variant::Basic => (1, 5),
            Variant::Mage(_) => (5, 5),
        }
    }
}
impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Basic => write!(f, "basic"),
            Variant::Mage(_) => write!(f, "mage"),
        }
    }
}
impl std::str::FromStr for Variant {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "basic" => Ok(Variant::Basic),
            "mage" => Ok(Variant::Mage(Vector::new(0, 0))),
            _ => Err("invalid variant".to_string()),
        }
    }
}
