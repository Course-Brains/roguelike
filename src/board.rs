use crate::{
    Enemy, Entity, Player, Random, Style, Vector,
    input::Direction,
    pieces::{door::Door, exit::Exit, item::Item, upgrade::Upgrade, wall::Wall},
    spell::{Spell, SpellCircle},
};
use albatrice::{FromBinary, ToBinary};
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
    pub spells: Vec<SpellCircle>,
    // Stuff that doesn't get calculated but does get drawn
    pub specials: Vec<Weak<Special>>,
    pub boss_pos: Vector,
    pub boss: Option<Weak<RwLock<Enemy>>>,
    visible: Vec<bool>,
    seen: Vec<bool>,
}
// General use
impl Board {
    pub fn new(x: usize, y: usize, render_x: usize, render_y: usize) -> Board {
        let mut inner = Vec::with_capacity(x * y);
        inner.resize_with(x * y, || None);
        let backtraces = vec![BackTrace::new(); x * y];
        let visible = vec![false; x * y];
        let seen = vec![false; x * y];
        Board {
            x,
            y,
            render_x,
            render_y,
            inner,
            enemies: Vec::new(),
            spells: Vec::new(),
            backtraces,
            specials: Vec::new(),
            boss_pos: Vector::new(0, 0),
            boss: None,
            visible,
            seen,
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
        if candidates.is_empty() {
            return None;
        }
        crate::random::random_index(candidates.len()).map(|index| candidates.swap_remove(index))
    }
    pub fn get_enemy(&self, pos: Vector, addr: Option<usize>) -> Option<Arc<RwLock<Enemy>>> {
        for enemy in self.enemies.iter() {
            if let Some(addr) = addr {
                if Arc::as_ptr(enemy).addr() == addr {
                    continue;
                }
            }
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
            if x % 10 == 0 {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                out[Vector::new(x, 28)] = Some(Piece::Upgrade(Upgrade::new(None)));
            }
        }
        out[Vector::new(1, 14)] = Some(Piece::Upgrade(Upgrade::new(Some(
            crate::upgrades::UpgradeType::SavePint,
        ))));
        out
    }
    pub fn contact_spell_at(&self, pos: Vector) -> Option<(&SpellCircle, usize)> {
        for (index, circle) in self.spells.iter().enumerate() {
            if circle.pos == pos {
                if let Spell::Contact(_) = circle.spell {
                    return Some((circle, index));
                }
            }
        }
        None
    }
    pub fn contains_literally_anything(&self, pos: Vector, addr: Option<usize>) -> bool {
        if self[pos].is_some() {
            return true;
        }
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
        for circle in self.spells.iter() {
            if circle.pos == pos {
                return true;
            }
        }
        false
    }
}
// Rendering
impl Board {
    // returns whether or not the cursor has a background behind it
    pub fn render(&self, bounds: Range<Vector>, lock: &mut impl Write, player: &Player) {
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
                    let index = self.to_index(Vector::new(x, y));
                    let mut memory = false;
                    if player.upgrades.map && piece.on_map() {
                        if !bounds.contains(&Vector::new(x, y)) {
                            continue;
                        }
                    } else if !self.visible[index] {
                        if self.seen[index] {
                            memory = true;
                        } else {
                            continue;
                        }
                    }
                    let (ch, mut style) = piece.render(Vector::new(x, y), self, player);
                    crossterm::queue!(
                        lock,
                        crossterm::cursor::MoveTo(
                            (x - x_bound.start) as u16,
                            (y - y_bound.start) as u16
                        )
                    )
                    .unwrap();
                    if memory {
                        match style {
                            Some(prev) => style = Some(*prev.clone().dim(true)),
                            None => style = Some(*Style::new().dim(true)),
                        }
                    }
                    if let Some(style) = style {
                        lock.write_fmt(format_args!("{style}{ch}\x1b[0m")).unwrap()
                    } else {
                        lock.write_fmt(format_args!("{ch}")).unwrap();
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
        self.render(bounds.clone(), &mut lock, player);
        self.draw_spells(&mut lock, bounds.clone());
        self.draw_enemies(&mut lock, bounds.clone(), player);
        self.draw_specials(&mut lock, bounds.clone());
        self.draw_desc(player, &mut lock);
        std::io::stdout().write_all(lock.make_contiguous()).unwrap();
        player.draw(self, bounds.clone());
        player.reposition_cursor(self.has_background(player.pos, player), bounds.clone());
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
                (ch, Some(style)) => write!(lock, "{style}{ch}\x1b[0m").unwrap(),
                (ch, None) => write!(lock, "{ch}").unwrap(),
            }
        }
    }
    pub fn is_visible(&self, pos: Vector, bounds: Range<Vector>) -> bool {
        if !bounds.contains(&pos) {
            return false;
        }
        self.visible[self.to_index(pos)]
    }
    fn draw_specials(&mut self, lock: &mut impl Write, bounds: Range<Vector>) {
        self.specials.retain(|special| special.upgrade().is_some());
        for special in self.specials.iter() {
            let special = special.upgrade().unwrap();
            if self.is_visible(special.pos, bounds.clone()) {
                crossterm::queue!(lock, (special.pos - bounds.start).to_move()).unwrap();
                match special.style {
                    Some(style) => write!(lock, "{}{}\x1b[0m", style, special.ch).unwrap(),
                    None => write!(lock, "{}", special.ch).unwrap(),
                }
            }
        }
    }
    fn draw_spells(&self, lock: &mut impl Write, bounds: Range<Vector>) {
        for spell in self.spells.iter() {
            if self.visible[self.to_index(spell.pos)] {
                crossterm::queue!(lock, (spell.pos - bounds.start).to_move()).unwrap();
                write!(lock, "{}∆\x1b[0m", Style::new().purple()).unwrap();
            }
        }
    }
    pub fn has_background(&self, pos: Vector, player: &Player) -> bool {
        if let Some(piece) = &self[pos] {
            if piece
                .render(pos, self, player)
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
        if !player.inspect {
            return;
        }
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
            } else if let Some(enemy) = self.get_enemy(player.selector, None) {
                write!(lock, "{}", enemy.try_read().unwrap().variant.kill_name()).unwrap()
            } else {
                match &self[player.selector] {
                    Some(piece) => piece.get_desc(lock),
                    None => write!(lock, "Nothing").unwrap(),
                }
            }
        } else if player.effects.mage_sight.is_active() {
            if let Some(enemy) = self.get_enemy(player.selector, None) {
                if enemy.try_read().unwrap().reachable {
                    write!(lock, ": {}", enemy.try_read().unwrap().variant.kill_name()).unwrap();
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
                    self.seen[index] = true;
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
    pub fn add_special(&mut self, special: Special) -> Arc<Special> {
        let arc = Arc::new(special);
        self.specials.push(Arc::downgrade(&arc));
        arc
    }
}
// Enemy logic
impl Board {
    pub fn generate_nav_data(&mut self, player: Vector) {
        if self.enemies.is_empty() {
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
            } else if piece.has_collision() {
                out.up = false
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Up).x) < 2
                && player.y.abs_diff((pos + Direction::Up).y) < 2
                && self.contains_enemy(pos + Direction::Up, None)
            {
                out.up = false
            }
        }

        if pos.y >= self.y - 1 {
            out.down = false
        } else if let Some(piece) = &self[pos + Direction::Down] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.down = false
                }
            } else if piece.has_collision() {
                out.down = false
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Down).x) < 2
                && player.y.abs_diff((pos + Direction::Down).y) < 2
                && self.contains_enemy(pos + Direction::Down, None)
            {
                out.down = false
            }
        }

        if pos.x == 0 {
            out.left = false
        } else if let Some(piece) = &self[pos + Direction::Left] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.left = false
                }
            } else if piece.has_collision() {
                out.left = false
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Left).x) < 2
                && player.y.abs_diff((pos + Direction::Left).y) < 2
                && self.contains_enemy(pos + Direction::Left, None)
            {
                out.left = false
            }
        }

        if pos.x >= self.x - 1 {
            out.right = false
        } else if let Some(piece) = &self[pos + Direction::Right] {
            if enemy_collision {
                if piece.enemy_collision() {
                    out.right = false
                }
            } else if piece.has_collision() {
                out.right = false
            }
        } else if let Some(player) = player {
            if player.x.abs_diff((pos + Direction::Right).x) < 2
                && player.y.abs_diff((pos + Direction::Right).y) < 2
                && self.contains_enemy(pos + Direction::Right, None)
            {
                out.right = false
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
    pub fn update_spells(&mut self, player: &mut Player) {
        let mut specials = Vec::new();
        for circle in self.spells.iter() {
            let arc = Arc::new(Special::new(circle.pos, '∆', Some(*Style::new().purple())));
            self.specials.push(Arc::downgrade(&arc));
            specials.push(arc);
        }
        let mut circles = std::mem::take(&mut self.spells);
        circles.retain(|circle| circle.update(self, player));
        self.spells = circles;
        std::mem::drop(specials);
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
impl FromBinary for Board {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Board {
            x: usize::from_binary(binary)?,
            y: usize::from_binary(binary)?,
            render_x: usize::from_binary(binary)?,
            render_y: usize::from_binary(binary)?,
            inner: Vec::from_binary(binary)?,
            backtraces: Vec::from_binary(binary)?,
            // cannot save outside of shop
            enemies: Vec::new(),
            // spells just can't be saved
            spells: Vec::new(),
            // Specials do not get maintained
            specials: Vec::new(),
            boss_pos: Vector::from_binary(binary)?,
            boss: None,
            visible: Vec::from_binary(binary)?,
            seen: Vec::from_binary(binary)?,
        })
    }
}
impl ToBinary for Board {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.x.to_binary(binary)?;
        self.y.to_binary(binary)?;
        self.render_x.to_binary(binary)?;
        self.render_y.to_binary(binary)?;
        self.inner
            .iter()
            .map(|x| x.as_ref())
            .collect::<Vec<Option<&Piece>>>()
            .to_binary(binary)?;
        self.backtraces.to_binary(binary)?;
        // skipping enemies
        // specials do not get saved
        self.boss_pos.to_binary(binary)?;
        // skipping boss because skipping enemies
        self.visible.to_binary(binary)?;
        self.seen.to_binary(binary)
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
impl FromBinary for BackTrace {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Self {
            from: crate::Direction::from_binary(binary)?,
            cost: Option::from_binary(binary)?,
        })
    }
}
impl ToBinary for BackTrace {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.from.to_binary(binary)?;
        self.cost.as_ref().to_binary(binary)
    }
}
#[derive(Clone, Debug)]
pub enum Piece {
    Wall(Wall),
    Door(Door),
    Exit(Exit),
    Item(Item),
    Upgrade(Upgrade),
}
impl Piece {
    fn render(&self, pos: Vector, board: &Board, player: &Player) -> (char, Option<Style>) {
        match self {
            Piece::Wall(_) => (Wall::render(pos, board), None),
            Piece::Door(door) => door.render(pos, board),
            Piece::Exit(_) => Exit::render(),
            Piece::Item(item) => item.render(player),
            Piece::Upgrade(upgrade) => upgrade.render(player),
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
        matches!(self, Piece::Wall(_) | Piece::Door(_))
    }
    pub fn enemy_collision(&self) -> bool {
        match self {
            Piece::Door(door) => door.has_collision(),
            _ => self.has_collision(),
        }
    }
    pub fn projectile_collision(&self) -> bool {
        match self {
            Piece::Door(door) => !door.open,
            Piece::Item(_) => false,
            Piece::Upgrade(_) => false,
            _ => true,
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
    pub fn on_step(&self, stepper: Entity<'_>) -> bool {
        match self {
            Piece::Exit(exit) => {
                exit.on_step(stepper);
                false
            }
            Piece::Item(item) => item.on_step(stepper),
            Piece::Upgrade(upgrade) => upgrade.on_step(stepper),
            _ => false,
        }
    }
    pub fn get_desc(&self, lock: &mut impl std::io::Write) {
        match self {
            Self::Wall(_) => write!(lock, "A wall").unwrap(),
            Self::Door(door) => door.get_desc(lock),
            Self::Exit(_) => write!(lock, "The exit").unwrap(),
            Self::Item(item) => item.get_desc(lock),
            Self::Upgrade(upgrade) => upgrade.get_desc(lock),
        }
    }
    pub fn on_map(&self) -> bool {
        matches!(self, Self::Wall(_) | Self::Door(_))
    }
}
impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Piece::Wall(wall) => wall.fmt(f),
            Piece::Door(door) => door.fmt(f),
            Piece::Exit(exit) => exit.fmt(f),
            Piece::Item(item) => item.fmt(f),
            Piece::Upgrade(upgrade) => upgrade.fmt(f),
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
                    "upgrade" => Ok(Piece::Upgrade(args.parse()?)),
                    invalid => Err(format!("{invalid} is not a valid piece type")),
                }
            }
            None => Err("You have to specify a piece type".to_string()),
        }
    }
}
impl FromBinary for Piece {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(match u8::from_binary(binary)? {
            0 => Self::Wall(Wall::from_binary(binary)?),
            1 => Self::Door(Door::from_binary(binary)?),
            2 => Self::Exit(Exit::from_binary(binary)?),
            3 => Self::Item(Item::from_binary(binary)?),
            4 => Self::Upgrade(Upgrade::from_binary(binary)?),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Could not get Piece from binary",
                ));
            }
        })
    }
}
impl ToBinary for Piece {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        match self {
            Self::Wall(wall) => {
                0_u8.to_binary(binary)?;
                wall.to_binary(binary)
            }
            Self::Door(door) => {
                1_u8.to_binary(binary)?;
                door.to_binary(binary)
            }
            Self::Exit(exit) => {
                2_u8.to_binary(binary)?;
                exit.to_binary(binary)
            }
            Self::Item(item) => {
                3_u8.to_binary(binary)?;
                item.to_binary(binary)
            }
            Self::Upgrade(upgrade) => {
                4_u8.to_binary(binary)?;
                upgrade.to_binary(binary)
            }
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
        Some(self.cmp(other))
    }
}
impl Ord for PathData {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self < other {
            std::cmp::Ordering::Less
        } else if self > other {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
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
#[derive(Clone, Copy)]
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
    pub pos: Vector,
    pub ch: char,
    pub style: Option<Style>,
}
impl Special {
    pub fn new(pos: Vector, ch: char, style: Option<Style>) -> Special {
        Special { pos, ch, style }
    }
}
impl FromBinary for Special {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        Ok(Special {
            pos: Vector::from_binary(binary)?,
            ch: char::from_binary(binary)?,
            style: Option::from_binary(binary)?,
        })
    }
}
impl ToBinary for Special {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        self.pos.to_binary(binary)?;
        self.ch.to_binary(binary)?;
        self.style.as_ref().to_binary(binary)
    }
}
