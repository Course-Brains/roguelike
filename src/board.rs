use crate::{
    Enemy, Player, Random, Style, Vector,
    input::Direction,
    pieces::{
        door::Door,
        exit::Exit,
        item::Item,
        spell::{Spell, Stepper},
        wall::Wall,
    },
};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::io::Write;
use std::ops::Range;
use std::sync::RwLock;
use std::sync::{Arc, Weak};
pub struct Board {
    pub x: usize,
    pub y: usize,
    pub render_x: usize, // the offset from the player to the edge of the screen(think radius)
    pub render_y: usize,
    pub inner: Vec<Option<Piece>>,
    pub backtraces: Vec<BackTrace>,
    pub enemies: Vec<Arc<RwLock<Enemy>>>,
    // Stuff that doesn't get calculated but does get drawn
    pub specials: Vec<Special>,
    boss_pos: Vector,
    pub boss: Option<Weak<RwLock<Enemy>>>,
    visible: Vec<bool>,
}
// General use
impl Board {
    pub fn new(x: usize, y: usize, render_x: usize, render_y: usize) -> Board {
        let mut inner = Vec::with_capacity(x * y);
        inner.resize_with(x * y, || None);
        let backtraces = vec![BackTrace::new(); x * y];
        let visible = vec![false; x * y];
        Board {
            x,
            y,
            render_x,
            render_y,
            inner,
            enemies: Vec::new(),
            backtraces,
            specials: Vec::new(),
            boss_pos: Vector::new(0, 0),
            boss: None,
            visible,
        }
    }
    pub fn to_index(&self, pos: Vector) -> usize {
        pos.y * self.x + pos.x
    }
    pub fn make_room(&mut self, point_1: Vector, point_2: Vector) {
        for x in point_1.x..point_2.x {
            self[Vector::new(x, point_1.y)] = Some(Piece::Wall(Wall {}));
            self[Vector::new(x, point_2.y - 1)] = Some(Piece::Wall(Wall {}));
        }
        for y in point_1.y..point_2.y {
            self[Vector::new(point_1.x, y)] = Some(Piece::Wall(Wall {}));
            self[Vector::new(point_2.x - 1, y)] = Some(Piece::Wall(Wall {}));
        }
    }
    pub fn has_collision(&self, pos: Vector) -> bool {
        if let Some(piece) = &self[pos] {
            if piece.has_collision() {
                return true;
            }
        }
        for enemy in self.enemies.iter() {
            if enemy.try_read().unwrap().pos == pos {
                return true;
            }
        }
        false
    }
    pub fn enemy_collision(&self, pos: Vector) -> bool {
        self[pos]
            .as_ref()
            .is_some_and(|piece| piece.enemy_collision())
    }
    pub fn dashable(&self, pos: Vector) -> bool {
        if let Some(piece) = &self[pos] {
            if !piece.dashable() {
                return false;
            }
        }
        true
    }
    pub fn get_near(
        &self,
        addr: Option<usize>,
        pos: Vector,
        range: usize,
    ) -> Vec<Weak<RwLock<Enemy>>> {
        let mut out = Vec::new();
        for enemy in self.enemies.iter() {
            if let Some(addr) = addr {
                if Arc::as_ptr(enemy).addr() == addr {
                    continue;
                }
            }
            if enemy.try_read().unwrap().is_near(pos, range) {
                out.push(Arc::downgrade(enemy))
            }
        }
        out
    }
    pub fn pick_near(
        &self,
        addr: Option<usize>,
        pos: Vector,
        range: usize,
    ) -> Option<Weak<RwLock<Enemy>>> {
        let mut candidates = self.get_near(addr, pos, range);
        if candidates.len() == 0 {
            return None;
        }
        crate::random::random_index(candidates.len()).map(|index| candidates.swap_remove(index))
    }
    pub fn get_enemy(&self, pos: Vector) -> Option<Arc<RwLock<Enemy>>> {
        for enemy in self.enemies.iter() {
            if enemy.try_read().unwrap().pos == pos {
                return Some(enemy.clone());
            }
        }
        None
    }
    pub fn new_shop() -> Board {
        let mut out = Board::new(90, 30, 45, 15);
        out.make_room(Vector::new(0, 0), Vector::new(90, 30));
        out[Vector::new(88, 15)] = Some(Piece::Exit(Exit::Level));
        for x in 1..=88 {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            out[Vector::new(x, 1)] = Some(Piece::Item(Item::new(None)));
            std::thread::sleep(std::time::Duration::from_millis(1000));
            out[Vector::new(x, 28)] = Some(Piece::Item(Item::new(None)));
        }
        out
    }
}
// Rendering
impl Board {
    // returns whether or not the cursor has a background behind it
    pub fn render(&self, bounds: Range<Vector>, lock: &mut impl Write) {
        let x_bound = bounds.start.x..bounds.end.x;
        let y_bound = bounds.start.y..bounds.end.y;
        for y in y_bound.clone() {
            crossterm::queue!(
                lock,
                crossterm::cursor::MoveTo(0, (y - y_bound.start) as u16)
            )
            .unwrap();
            write!(lock, "\x1b[2K").unwrap();
            for x in x_bound.clone() {
                if let Some(piece) = &self[Vector::new(x, y)] {
                    if !self.visible[self.to_index(Vector::new(x, y))] {
                        continue;
                    }
                    let (ch, style) = piece.render(Vector::new(x, y), self);
                    crossterm::queue!(
                        lock,
                        crossterm::cursor::MoveTo(
                            (x - x_bound.start) as u16,
                            (y - y_bound.start) as u16
                        )
                    )
                    .unwrap();
                    if let Some(style) = style {
                        lock.write_fmt(format_args!("{}{}\x1b[0m", style.enact(), ch))
                            .unwrap()
                    } else {
                        lock.write_fmt(format_args!("{}", ch)).unwrap();
                    }
                }
            }
        }
        write!(lock, "\x1b[B\x1b[2K").unwrap();
    }
    pub fn smart_render(&mut self, player: &mut Player) {
        let mut lock = VecDeque::new();
        let bounds = self.get_render_bounds(player);
        crossterm::queue!(
            std::io::stdout(),
            crossterm::terminal::BeginSynchronizedUpdate
        )
        .unwrap();
        self.generate_visible(player);
        self.render(bounds.clone(), &mut lock);
        self.draw_enemies(&mut lock, bounds.clone(), player);
        self.draw_specials(&mut lock, bounds.clone());
        self.draw_desc(player, &mut lock);
        std::io::stdout().write_all(lock.make_contiguous()).unwrap();
        player.draw(self, bounds.clone());
        player.reposition_cursor(self.has_background(player.pos), bounds.clone());
        crossterm::queue!(
            std::io::stdout(),
            crossterm::terminal::EndSynchronizedUpdate
        )
        .unwrap();
        std::io::stdout().flush().unwrap();
    }
    fn draw_enemies(&self, lock: &mut impl Write, bounds: Range<Vector>, player: &Player) {
        for enemy in self.enemies.iter() {
            let pos = enemy.try_read().unwrap().pos;
            if !bounds.contains(&pos) {
                continue;
            }
            if player.effects.mage_sight.is_active() {
                if !enemy.try_read().unwrap().reachable {
                    continue;
                }
            } else if !self.visible[self.to_index(pos)] {
                continue;
            }
            crossterm::queue!(lock, (pos - bounds.start).to_move()).unwrap();
            match enemy.try_read().unwrap().render() {
                (ch, Some(style)) => write!(lock, "{}{ch}\x1b[0m", style.enact()).unwrap(),
                (ch, None) => write!(lock, "{ch}").unwrap(),
            }
        }
    }
    fn is_visible(&self, pos: Vector, bounds: Range<Vector>) -> bool {
        if !bounds.contains(&pos) {
            return false;
        }
        self.visible[self.to_index(pos)]
    }
    fn draw_specials(&self, lock: &mut impl Write, bounds: Range<Vector>) {
        for special in self.specials.iter() {
            if bounds.contains(&special.pos) {
                crossterm::queue!(lock, (special.pos - bounds.start).to_move()).unwrap();
                match special.style {
                    Some(style) => write!(lock, "{}{}\x1b[0m", style.enact(), special.ch).unwrap(),
                    None => write!(lock, "{}", special.ch).unwrap(),
                }
            }
        }
    }
    pub fn has_background(&self, pos: Vector) -> bool {
        if let Some(piece) = &self[pos] {
            if piece
                .render(pos, self)
                .1
                .is_some_and(|x| x.has_background())
            {
                return true;
            }
        }
        for enemy in self.enemies.iter() {
            if enemy.try_read().unwrap().pos == pos {
                if let Some(style) = enemy.try_read().unwrap().render().1 {
                    if style.has_background() {
                        return true;
                    }
                }
                break;
            }
        }
        false
    }
    pub fn get_render_bounds(&self, player: &Player) -> Range<Vector> {
        let mut base = Vector::new(0, 0);
        let pos = player.get_focus();
        if pos.x < self.render_x {
            base.x = 0
        } else if pos.x > self.x - self.render_x {
            base.x = self.x - self.render_x * 2
        } else {
            base.x = pos.x - self.render_x
        }
        if pos.y < self.render_y {
            base.y = 0
        } else if pos.y > self.y - self.render_y {
            base.y = self.y - self.render_y * 2
        } else {
            base.y = pos.y - self.render_y
        }
        base..Vector::new(base.x + self.render_x * 2, base.y + self.render_y * 2)
    }
    pub fn draw_desc(&self, player: &Player, lock: &mut impl Write) {
        Board::go_to_desc(lock);
        crossterm::queue!(
            lock,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        )
        .unwrap();
        write!(lock, " {}", player.selector).unwrap();
        if self.visible[self.to_index(player.selector)] {
            write!(lock, ": ").unwrap();
            // Player -> Enemies -> Map elements
            if player.pos == player.selector {
                write!(lock, "You").unwrap();
            } else if let Some(enemy) = self.get_enemy(player.selector) {
                write!(lock, "{}", enemy.try_read().unwrap().variant.kill_name()).unwrap()
            } else {
                match &self[player.selector] {
                    Some(piece) => piece.get_desc(lock),
                    None => write!(lock, "Nothing").unwrap(),
                }
            }
        } else {
            if player.effects.mage_sight.is_active() {
                if let Some(enemy) = self.get_enemy(player.selector) {
                    if enemy.try_read().unwrap().reachable {
                        write!(lock, ": {}", enemy.try_read().unwrap().variant.kill_name())
                            .unwrap();
                    }
                }
            }
        }
    }
    pub fn go_to_desc(lock: &mut impl Write) {
        crossterm::queue!(lock, crossterm::cursor::MoveTo(0, 93),).unwrap();
    }
    pub fn set_desc(lock: &mut impl Write, text: &str) {
        Board::go_to_desc(lock);
        crossterm::queue!(
            lock,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        )
        .unwrap();
        write!(lock, "{text}").unwrap()
    }
    fn generate_visible(&mut self, player: &Player) {
        let mut start = std::time::Instant::now();
        for vis in self.visible.iter_mut() {
            *vis = false;
        }
        let elapsed = start.elapsed();
        crate::log!(
            "Vis clear time: {}ms, {}ns",
            elapsed.as_millis(),
            elapsed.as_nanos()
        );
        start = std::time::Instant::now();
        let mut next = VecDeque::new();
        let mut seen = HashSet::new();
        seen.insert(player.pos);
        next.push_front((player.pos, 0));
        while let Some((pos, cost)) = next.pop_back() {
            if cost > player.perception {
                continue;
            }
            for x in -1..=1 {
                for y in -1..=1 {
                    let index = self.to_index(Vector::new(
                        (pos.x as isize + x) as usize,
                        (pos.y as isize + y) as usize,
                    ));
                    self.visible[index] = true;
                }
            }
            for pos in self.get_adjacent(pos, None, false).to_vec(pos).iter() {
                if !seen.contains(pos) {
                    seen.insert(*pos);
                    next.push_front((*pos, cost + 1));
                }
            }
        }
        let elapsed = start.elapsed();
        crate::log!(
            "Vis calc time: {}ms ({}ns)",
            elapsed.as_millis(),
            elapsed.as_nanos()
        );
    }
}
// Enemy logic
impl Board {
    pub fn generate_nav_data(&mut self, player: Vector) {
        if self.enemies.len() == 0 {
            return;
        }
        let start = std::time::Instant::now();
        for item in self.backtraces.iter_mut() {
            item.cost = None;
        }
        let elapsed = start.elapsed();
        crate::log!(
            "clear path time: {}({})",
            elapsed.as_millis(),
            elapsed.as_nanos()
        );
        let start = std::time::Instant::now();
        let mut to_visit: BinaryHeap<PathData> = BinaryHeap::new();
        let mut visited = HashSet::new();
        to_visit.push(PathData::new(
            Direction::Up,
            player,
            self.enemies[0].try_read().unwrap().pos,
            0,
        ));
        for enemy in self.enemies.iter() {
            if !enemy.try_read().unwrap().reachable {
                continue;
            }
            if self.backtraces[self.to_index(enemy.try_read().unwrap().pos)]
                .cost
                .is_some()
            {
                continue;
            }
            to_visit = to_visit
                .iter()
                .map(|item| {
                    PathData::new(
                        item.from,
                        item.pos,
                        enemy.try_read().unwrap().pos,
                        item.cost,
                    )
                })
                .collect();

            while let Some(path_data) = to_visit.pop() {
                let index = self.to_index(path_data.pos);
                if self.backtraces[index]
                    .cost
                    .is_none_or(|cost| cost > path_data.cost)
                {
                    self.backtraces[index].cost = Some(path_data.cost);
                    self.backtraces[index].from = path_data.from;
                }
                if path_data.pos == enemy.try_read().unwrap().pos {
                    break;
                }
                if visited.contains(&path_data.pos) {
                    continue;
                }
                visited.insert(path_data.pos);
                let adj = self.get_adjacent(path_data.pos, Some(player), true);
                if adj.up {
                    to_visit.push(PathData::new(
                        Direction::Down,
                        path_data.pos + Direction::Up,
                        enemy.try_read().unwrap().pos,
                        path_data.cost + 1,
                    ));
                }
                if adj.down {
                    to_visit.push(PathData::new(
                        Direction::Up,
                        path_data.pos + Direction::Down,
                        enemy.try_read().unwrap().pos,
                        path_data.cost + 1,
                    ))
                }
                if adj.left {
                    to_visit.push(PathData::new(
                        Direction::Right,
                        path_data.pos + Direction::Left,
                        enemy.try_read().unwrap().pos,
                        path_data.cost + 1,
                    ))
                }
                if adj.right {
                    to_visit.push(PathData::new(
                        Direction::Left,
                        path_data.pos + Direction::Right,
                        enemy.try_read().unwrap().pos,
                        path_data.cost + 1,
                    ))
                }
            }
        }
        let elapsed = start.elapsed();
        crate::log!(
            "path calc time: {}({})",
            elapsed.as_millis(),
            elapsed.as_nanos()
        );
    }
    pub fn get_adjacent(&self, pos: Vector, player: Option<Vector>, enemy_collision: bool) -> Adj {
        let mut out = Adj::new(true);

        if pos.y == 0 {
            out.up = false
        } else if let Some(piece) = &self[pos + Direction::Up] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.up = false
                }
            } else {
                if piece.has_collision() {
                    out.up = false
                }
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Up).x) < 2 {
                if player.y.abs_diff((pos + Direction::Up).y) < 2 {
                    if self.contains_enemy(pos + Direction::Up, None) {
                        out.up = false
                    }
                }
            }
        }

        if pos.y >= self.y - 1 {
            out.down = false
        } else if let Some(piece) = &self[pos + Direction::Down] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.down = false
                }
            } else {
                if piece.has_collision() {
                    out.down = false
                }
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Down).x) < 2 {
                if player.y.abs_diff((pos + Direction::Down).y) < 2 {
                    if self.contains_enemy(pos + Direction::Down, None) {
                        out.down = false
                    }
                }
            }
        }

        if pos.x == 0 {
            out.left = false
        } else if let Some(piece) = &self[pos + Direction::Left] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.left = false
                }
            } else {
                if piece.has_collision() {
                    out.left = false
                }
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Left).x) < 2 {
                if player.y.abs_diff((pos + Direction::Left).y) < 2 {
                    if self.contains_enemy(pos + Direction::Left, None) {
                        out.left = false
                    }
                }
            }
        }

        if pos.x >= self.x - 1 {
            out.right = false
        } else if let Some(piece) = &self[pos + Direction::Right] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.right = false
                }
            } else {
                if piece.has_collision() {
                    out.right = false
                }
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Right).x) < 2 {
                if player.y.abs_diff((pos + Direction::Right).y) < 2 {
                    if self.contains_enemy(pos + Direction::Right, None) {
                        out.right = false
                    }
                }
            }
        }

        out
    }
    pub fn flood(&mut self, player: Vector) {
        let start = std::time::Instant::now();
        let mut lookup = HashMap::new();
        for (index, enemy) in self.enemies.iter_mut().enumerate() {
            enemy.try_write().unwrap().reachable = false;
            lookup.insert(enemy.try_read().unwrap().pos, index);
        }
        let mut to_visit = VecDeque::new();
        let mut seen = HashSet::new();
        to_visit.push_front(player);
        seen.insert(player);
        while let Some(pos) = to_visit.pop_back() {
            if let Some(index) = lookup.get(&pos) {
                self.enemies[*index].try_write().unwrap().reachable = true;
            }
            let adj = self.get_adjacent(pos, None, false);
            if adj.up {
                if pos.y == 0 {
                    crate::log!("up at {pos}")
                }
                if !seen.contains(&(pos + Direction::Up)) {
                    to_visit.push_front(pos + Direction::Up);
                    seen.insert(pos + Direction::Up);
                }
            }
            if adj.down {
                if pos.y >= self.y - 1 {
                    crate::log!("down at {pos}")
                }
                if !seen.contains(&(pos + Direction::Down)) {
                    to_visit.push_front(pos + Direction::Down);
                    seen.insert(pos + Direction::Down);
                }
            }
            if adj.left {
                if pos.x == 0 {
                    crate::log!("left at {pos}")
                }
                if !seen.contains(&(pos + Direction::Left)) {
                    to_visit.push_front(pos + Direction::Left);
                    seen.insert(pos + Direction::Left);
                }
            }
            if adj.right {
                if pos.x >= self.x - 1 {
                    crate::log!("right at {pos}")
                }
                if !seen.contains(&(pos + Direction::Right)) {
                    to_visit.push_front(pos + Direction::Right);
                    seen.insert(pos + Direction::Right);
                }
            }
        }
        let elapsed = start.elapsed();
        crate::log!(
            "flood time: {}({})",
            elapsed.as_millis(),
            elapsed.as_nanos()
        )
    }
    pub fn move_and_think(
        &mut self,
        player: &mut Player,
        enemy: Arc<RwLock<Enemy>>,
        bounds: Range<Vector>,
    ) {
        if self.move_enemy(player, enemy.clone())
            && self.is_visible(enemy.try_read().unwrap().pos, bounds.clone())
        {
            self.smart_render(player);
            std::thread::sleep(crate::DELAY);
        }
        if Enemy::think(enemy.clone(), self, player)
            && self.is_visible(enemy.try_read().unwrap().pos, bounds.clone())
        {
            self.smart_render(player);
            std::thread::sleep(crate::DELAY);
        }
    }
    pub fn move_enemy(&mut self, player: &mut Player, arc: Arc<RwLock<Enemy>>) -> bool {
        let mut enemy = arc.try_write().unwrap();
        let addr = Arc::as_ptr(&arc).addr();
        if !enemy.active || !enemy.reachable || enemy.attacking || enemy.is_stunned() {
            return false;
        }
        enemy.alert_nearby(addr, self, crate::random() as usize & 7);
        let mut new_dir = self.backtraces[self.to_index(enemy.pos)].from;
        if self.contains_enemy(enemy.pos + new_dir, Some(addr))
            || crate::random() & 0b0001_1111 == 0
        {
            match new_dir {
                Direction::Up | Direction::Down => {
                    if bool::random() {
                        new_dir = Direction::Left
                    } else {
                        new_dir = Direction::Right
                    }
                }
                Direction::Left | Direction::Right => {
                    if bool::random() {
                        new_dir = Direction::Up
                    } else {
                        new_dir = Direction::Down
                    }
                }
            }
        }
        let new_pos = enemy.pos + new_dir;
        if self.enemy_collision(new_pos) {
            return false;
        }
        if self.contains_enemy(new_pos, Some(addr)) {
            return false;
        }
        if player.pos == new_pos {
            return false;
        }
        enemy.pos = new_pos;
        if let Some(boss) = self.boss.as_ref().unwrap().upgrade() {
            if Arc::ptr_eq(&boss, &arc) {
                self.boss_pos = enemy.pos;
            }
        }
        true
    }
    fn contains_enemy(&self, pos: Vector, addr: Option<usize>) -> bool {
        for enemy in self.enemies.iter() {
            if let Some(addr) = addr {
                if Arc::as_ptr(enemy).addr() == addr {
                    continue;
                }
            }
            if enemy.try_read().unwrap().pos == pos {
                return true;
            }
        }
        false
    }
    pub fn purge_dead(&mut self) {
        self.enemies.retain(|enemy| !enemy.try_read().unwrap().dead);
    }
    // places the exit if the boss is dead
    pub fn place_exit(&mut self) {
        if let Some(boss) = self.boss.as_ref() {
            if boss.upgrade().is_none() {
                let pos = self.boss_pos;
                self[pos] = Some(Piece::Exit(Exit::Shop));
            }
        }
    }
}
impl std::ops::Index<Vector> for Board {
    type Output = Option<Piece>;
    fn index(&self, index: Vector) -> &Self::Output {
        &self.inner[self.to_index(index)]
    }
}
impl std::ops::IndexMut<Vector> for Board {
    fn index_mut(&mut self, index: Vector) -> &mut Self::Output {
        &mut self.inner[index.y * self.x + index.x]
    }
}
#[derive(Copy, Clone)]
pub struct BackTrace {
    from: crate::Direction,
    pub cost: Option<usize>,
}
impl BackTrace {
    const fn new() -> BackTrace {
        BackTrace {
            from: crate::Direction::Up,
            cost: None,
        }
    }
}
#[derive(Clone, Debug)]
pub enum Piece {
    Wall(Wall),
    Door(Door),
    Spell(Spell),
    Exit(Exit),
    Item(Item),
}
impl Piece {
    fn render(&self, pos: Vector, board: &Board) -> (char, Option<Style>) {
        match self {
            Piece::Wall(_) => (Wall::render(pos, board), None),
            Piece::Door(door) => door.render(pos, board),
            Piece::Spell(_) => (Spell::SYMBOL, Some(Spell::STYLE)),
            Piece::Exit(_) => Exit::render(),
            Piece::Item(_) => Item::render(),
        }
    }
    pub fn has_collision(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(door) => door.has_collision(),
            _ => false,
        }
    }
    pub fn wall_connectable(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(_) => true,
            _ => false,
        }
    }
    pub fn enemy_collision(&self) -> bool {
        match self {
            Piece::Wall(_) => true,
            Piece::Door(door) => door.has_collision(),
            Piece::Spell(_) => true,
            Piece::Exit(_) => false,
            Piece::Item(_) => false,
        }
    }
    fn dashable(&self) -> bool {
        match self {
            Piece::Wall(_) => false,
            Piece::Door(door) => !door.has_collision(),
            _ => true,
        }
    }
    // returns whether or not the piece should be deleted
    pub fn on_step(&self, stepper: Stepper<'_>) -> bool {
        match self {
            Piece::Spell(spell) => {
                spell.on_step(stepper);
                true
            }
            Piece::Exit(exit) => {
                exit.on_step(stepper);
                false
            }
            Piece::Item(item) => item.on_step(stepper),
            _ => false,
        }
    }
    pub fn get_desc(&self, lock: &mut impl std::io::Write) {
        match self {
            Self::Wall(_) => write!(lock, "A wall").unwrap(),
            Self::Door(door) => door.get_desc(lock),
            Self::Spell(_) => write!(lock, "A spell").unwrap(),
            Self::Exit(_) => write!(lock, "The exit").unwrap(),
            Self::Item(item) => item.get_desc(lock),
        }
    }
}
impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Piece::Wall(wall) => wall.fmt(f),
            Piece::Door(door) => door.fmt(f),
            Piece::Spell(spell) => spell.fmt(f),
            Piece::Exit(exit) => exit.fmt(f),
            Piece::Item(item) => item.fmt(f),
        }
    }
}
impl std::str::FromStr for Piece {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(' ');
        match split.next() {
            Some(piece_type) => {
                let args: String = split.map(|s| s.to_string() + " ").collect();
                match piece_type {
                    "wall" => Ok(Piece::Wall(Wall {})),
                    "door" => Ok(Piece::Door(args.parse()?)),
                    "exit" => Ok(Piece::Exit(args.parse()?)),
                    "item" => Ok(Piece::Item(args.parse()?)),
                    "spell" => Err("Spells cannot be created like this".to_string()),
                    invalid => Err(format!("{invalid} is not a valid piece type")),
                }
            }
            None => Err("You have to specify a piece type".to_string()),
        }
    }
}
struct PathData {
    from: Direction,
    cost: usize,
    heur: usize,
    pos: Vector,
}
impl PathData {
    fn new(from: Direction, pos: Vector, target: Vector, cost: usize) -> PathData {
        PathData {
            from,
            cost,
            heur: cost + pos.x.abs_diff(target.x) + pos.y.abs_diff(target.y),
            pos,
        }
    }
}
impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        self.heur == other.heur
    }
    fn ne(&self, other: &Self) -> bool {
        self.heur != other.heur
    }
}
impl Eq for PathData {}
impl PartialOrd for PathData {
    fn lt(&self, other: &Self) -> bool {
        self.heur.gt(&other.heur)
    }
    fn le(&self, other: &Self) -> bool {
        self.heur.ge(&other.heur)
    }
    fn gt(&self, other: &Self) -> bool {
        self.heur.lt(&other.heur)
    }
    fn ge(&self, other: &Self) -> bool {
        self.heur.le(&other.heur)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self < other {
            Some(std::cmp::Ordering::Less)
        } else if self > other {
            Some(std::cmp::Ordering::Greater)
        } else if self == other {
            Some(std::cmp::Ordering::Equal)
        } else {
            None
        }
    }
}
impl Ord for PathData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if self > other { self } else { other }
    }
    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        if self < other { self } else { other }
    }
    fn clamp(self, _min: Self, _max: Self) -> Self
    where
        Self: Sized,
    {
        unimplemented!("don't")
    }
}
pub struct Adj {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}
impl Adj {
    fn new(state: bool) -> Adj {
        Adj {
            up: state,
            down: state,
            left: state,
            right: state,
        }
    }
    pub fn to_vec(self, base: Vector) -> Vec<Vector> {
        let mut out = Vec::new();
        if self.up {
            out.push(base + Direction::Up)
        }
        if self.down {
            out.push(base + Direction::Down)
        }
        if self.left {
            out.push(base + Direction::Left)
        }
        if self.right {
            out.push(base + Direction::Right)
        }
        out
    }
}
pub struct Special {
    pos: Vector,
    ch: char,
    style: Option<Style>,
}
impl Special {
    pub fn new(pos: Vector, ch: char, style: Option<Style>) -> Special {
        Special { pos, ch, style }
    }
}
