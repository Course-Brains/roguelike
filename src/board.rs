use crate::{
    Enemy, Entity, Player, Random, Style, Vector,
    input::Direction,
    pieces::{door::Door, exit::Exit, item::Item, upgrade::Upgrade, wall::Wall},
    spell::{Spell, SpellCircle},
};
use abes_nice_things::{FromBinary, ToBinary};
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::io::{Read, Write};
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
    pub bosses: Vec<Boss>,
    pub visible: Vec<bool>,
    pub seen: Vec<bool>,
    // The number of turns spent on this board specifically
    pub turns_spent: usize,
    pub reachable: Vec<bool>,
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
            bosses: Vec::new(),
            visible,
            seen,
            turns_spent: 0,
            reachable: vec![false; x * y],
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
    // player pos will be 44 14
    pub const BONUS_NO_DAMAGE: Vector = Vector::new(43, 13);
    pub const BONUS_NO_WASTE: Vector = Vector::new(45, 13);
    pub const BONUS_KILL_ALL: Vector = Vector::new(43, 15);
    pub const BONUS_NO_ENERGY: Vector = Vector::new(45, 15);
    pub fn new_shop() -> Board {
        let mut out = Board::new(90, 30, 45, 15);

        // Creating room
        out.make_room(Vector::new(0, 0), Vector::new(90, 30));

        // Placing exit
        out[Vector::new(88, 15)] = Some(Piece::Exit(Exit::Level));

        // Placing items
        for x in 1..=88 {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            out[Vector::new(x, 1)] = Some(Piece::Item(Item::new(None)));
        }

        // Placing upgrades/limbs
        let mut available = crate::SHOP_AVAILABILITY.lock().unwrap().1.recv().unwrap();
        for x in 1..=88 {
            if x % 10 == 0 {
                let chosen = {
                    // Assume that the list of available upgrades is creater than 0
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    let index = crate::random::random_index(available.len()).unwrap();
                    if available[index].is_repeatable().unwrap() {
                        available[index]
                    } else {
                        available.swap_remove(index)
                    }
                };
                out[Vector::new(x, 28)] = Some(Piece::Upgrade(Upgrade::new(Some(chosen))));
            }
        }

        // Placing save pint
        out[Vector::new(1, 14)] = Some(Piece::Upgrade(Upgrade::new(Some(
            crate::upgrades::UpgradeType::SavePint,
        ))));

        // Placing Bonus fail signs(will be replaced on load if condition is met)
        // Take no damage
        out[Board::BONUS_NO_DAMAGE] = Some(Piece::Sign("Don't get hurt".to_string()));
        // Waste no kill rewards, might make it easier at some point because it can be impossible
        // to get both this and the no damage bonus
        out[Board::BONUS_NO_WASTE] = Some(Piece::Sign("Be a miser".to_string()));
        // Kill every enemy
        out[Board::BONUS_KILL_ALL] = Some(Piece::Sign("Be more thorough".to_string()));
        // Spend no energy directly(not counting conversion)
        out[Board::BONUS_NO_ENERGY] = Some(Piece::Sign("Be more stingy".to_string()));

        // Placing limbs

        out
    }
    pub fn new_empty() -> Board {
        let mut out = Board::new(90, 30, 45, 15);
        out.make_room(Vector::new(0, 0), Vector::new(90, 30));
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
    pub fn flood_within(&self, origin: Vector, range: usize, enemy_collision: bool) -> Vec<Vector> {
        let mut seen = HashSet::new();
        let mut next = VecDeque::new();
        next.push_front((origin, 0));
        seen.insert(origin);
        while let Some((pos, cost)) = next.pop_back() {
            if cost >= range {
                continue;
            }
            for adj in self
                .get_adjacent(pos, None, enemy_collision)
                .to_vec(pos)
                .iter()
            {
                if !seen.contains(adj) {
                    seen.insert(*adj);
                    next.push_front((*adj, cost + 1));
                }
            }
        }
        seen.iter().map(|pos| *pos).collect()
    }
    pub fn is_reachable(&self, pos: Vector) -> bool {
        self.reachable[self.to_index(pos)]
    }
    pub fn get_visible_indexes(&self, bounds: Range<Vector>, full_vis: bool) -> Vec<usize> {
        let mut out = Vec::new();
        for (index, enemy) in self.enemies.iter().enumerate() {
            if self.is_visible(enemy.try_read().unwrap().pos, bounds.clone(), full_vis) {
                out.push(index)
            }
        }
        out
    }
    // Returns if there is an enemy aiming at the player, and if there is, then if there is an
    // enemy that is about to fire at the player
    pub fn is_enemy_aiming(&self) -> Option<bool> {
        let mut out = None;
        for arc in self.enemies.iter() {
            if let Some(urgent) = arc.try_read().unwrap().is_aiming() {
                if urgent {
                    out = Some(true);
                    break;
                } else if out.is_none() {
                    out = Some(false)
                }
            }
        }
        out
    }
    pub fn reset_took_damage(&self) {
        for arc in self.enemies.iter() {
            arc.try_write().unwrap().took_damage = false;
        }
    }
    pub fn get_highest_tier(&self) -> usize {
        let mut highest = 0;
        for enemy in self.enemies.iter() {
            if let Ok(tier) = enemy.try_read().unwrap().variant.get_tier() {
                if tier > highest {
                    highest = tier;
                }
            }
        }
        highest
    }
    pub fn get_all_of_tier(&self, target: usize, out: &mut Vec<Arc<RwLock<Enemy>>>) {
        for enemy in self.enemies.iter() {
            if let Ok(tier) = enemy.try_read().unwrap().variant.get_tier() {
                if target == tier {
                    out.push(enemy.clone());
                }
            }
        }
    }
}
// Rendering
impl Board {
    // returns whether or not the cursor has a background behind it
    pub fn render(&self, bounds: Range<Vector>, lock: &mut impl Write, player: &Player) {
        let start = if crate::bench() {
            Some(std::time::Instant::now())
        } else {
            None
        };
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
                    if player.effects.full_vis.is_active()
                        || (player.upgrades.map && piece.on_map())
                    {
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
        if let Some(start) = start {
            let elapsed = start.elapsed();
            writeln!(crate::bench::render(), "{}", elapsed.as_millis()).unwrap();
        }
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
        if player.limbs.count_seer_eyes() == 2 {
            self.draw_premonition(
                &mut lock,
                bounds.clone(),
                player.effects.full_vis.is_active(),
            )
        }
        self.draw_spells(
            &mut lock,
            bounds.clone(),
            player.effects.full_vis.is_active(),
        );
        self.draw_enemies(&mut lock, bounds.clone(), player);
        self.draw_specials(
            &mut lock,
            bounds.clone(),
            player.effects.full_vis.is_active(),
        );
        self.draw_desc(player, &mut lock);
        self.draw_feedback();
        if crate::show_reachable() {
            self.draw_reachable(&mut lock, bounds.clone());
        }
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
            let mage_eyes = player.limbs.count_mage_eyes();
            let directly_seen =
                player.effects.full_vis.is_active() || self.is_visible(pos, bounds.clone(), false);
            let magically_seen = mage_eyes > 0 && self.is_reachable(pos);
            let obfuscated = magically_seen && !directly_seen && mage_eyes == 1;
            if !(magically_seen || directly_seen) {
                continue;
            }

            crossterm::queue!(lock, (pos - bounds.start).to_move()).unwrap();
            if obfuscated {
                write!(lock, "?").unwrap();
            } else {
                match enemy.try_read().unwrap().render() {
                    (ch, Some(style)) => write!(lock, "{style}{ch}\x1b[0m").unwrap(),
                    (ch, None) => write!(lock, "{ch}").unwrap(),
                }
            }
        }
    }
    pub fn is_visible(&self, pos: Vector, bounds: Range<Vector>, full_vis: bool) -> bool {
        if !bounds.contains(&pos) {
            return false;
        }
        self.visible[self.to_index(pos)] || full_vis
    }
    fn draw_specials(&mut self, lock: &mut impl Write, bounds: Range<Vector>, full_vis: bool) {
        self.specials.retain(|special| special.upgrade().is_some());
        for special in self.specials.iter() {
            let special = special.upgrade().unwrap();
            if self.is_visible(special.pos, bounds.clone(), full_vis) {
                crossterm::queue!(lock, (special.pos - bounds.start).to_move()).unwrap();
                match special.style {
                    Some(style) => write!(lock, "{}{}\x1b[0m", style, special.ch).unwrap(),
                    None => write!(lock, "{}", special.ch).unwrap(),
                }
            }
        }
    }
    fn draw_spells(&self, lock: &mut impl Write, bounds: Range<Vector>, full_vis: bool) {
        for spell in self.spells.iter() {
            if self.is_visible(spell.pos, bounds.clone(), full_vis) {
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
        } else if player.limbs.count_mage_eyes() == 2 {
            if let Some(enemy) = self.get_enemy(player.selector, None) {
                if self.is_reachable(enemy.try_read().unwrap().pos) {
                    write!(lock, ": {}", enemy.try_read().unwrap().variant.kill_name()).unwrap();
                }
            }
        }
    }
    pub fn go_to_desc(lock: &mut impl Write) {
        crossterm::queue!(lock, crossterm::cursor::MoveTo(0, 36),).unwrap();
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
            if cost > player.get_perception() {
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
        if crate::bench() {
            writeln!(crate::bench::vis_flood(), "{}", elapsed.as_millis()).unwrap();
        }
    }
    pub fn add_special(&mut self, special: Special) -> Arc<Special> {
        let arc = Arc::new(special);
        self.specials.push(Arc::downgrade(&arc));
        arc
    }
    pub fn add_one_turn_special(&mut self, special: Special) {
        crate::ONE_TURN_SPECIALS
            .lock()
            .unwrap()
            .push(self.add_special(special));
    }
    pub fn draw_premonition(&self, lock: &mut impl Write, bounds: Range<Vector>, full_vis: bool) {
        let mut targets = Vec::new();
        for arc in self.enemies.iter() {
            arc.try_read().unwrap().get_aim_pos(&mut targets);
        }
        for (pos, urgent) in targets.iter() {
            if self.is_visible(*pos, bounds.clone(), full_vis) {
                crossterm::queue!(lock, (*pos - bounds.start).to_move()).unwrap();
                write!(
                    lock,
                    "{} \x1b[0m",
                    match urgent {
                        true => {
                            *Style::new().background_red().intense_background(true)
                        }
                        false => {
                            *Style::new().background_red()
                        }
                    }
                )
                .unwrap()
            }
        }
    }
    pub fn draw_feedback(&self) {
        crate::draw_feedback();
    }
    pub fn draw_reachable(&self, lock: &mut impl Write, bounds: Range<Vector>) {
        for x in bounds.start.x..bounds.end.x {
            for y in bounds.start.y..bounds.end.y {
                if self.is_reachable(Vector::new(x, y)) {
                    crossterm::queue!(lock, (Vector::new(x, y) - bounds.start).to_move()).unwrap();
                    write!(lock, "{} \x1b[0m", Style::new().background_green()).unwrap();
                }
            }
        }
    }
}
// Enemy logic
impl Board {
    pub fn generate_nav_data(
        &mut self,
        player: Vector,
        stepthrough: bool,
        stepthrough_index: Option<usize>,
        player_mut: &mut Player,
    ) {
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
        let mut specials = Vec::new();
        let mut skip_stepthrough = false;
        for (enemy_index, enemy) in self.enemies.clone().iter().enumerate() {
            let mut skip_stepthrough_single = false;
            if !self.is_reachable(enemy.try_read().unwrap().pos) {
                continue;
            }
            if enemy.try_read().unwrap().pos.is_near(player, 2) {
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
                if stepthrough
                    && !skip_stepthrough_single
                    && !skip_stepthrough
                    && stepthrough_index.is_none_or(|index| index == enemy_index)
                    && self.is_visible(
                        path_data.pos,
                        self.get_render_bounds(player_mut),
                        player_mut.effects.full_vis.is_active(),
                    )
                {
                    specials.push(self.add_special(Special::new(
                        path_data.pos,
                        ' ',
                        Some(*Style::new().background_green()),
                    )));
                    self.smart_render(player_mut);
                    let mut buf = [0];
                    std::io::stdin().read_exact(&mut buf).unwrap();
                    if buf[0] == b's' {
                        skip_stepthrough_single = true
                    } else if buf[0] == b'S' {
                        skip_stepthrough = true
                    }
                }
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
        if stepthrough {
            crate::log!(
                "path calc time: {}({}) [WITH STEPTHROUGH]",
                elapsed.as_millis(),
                elapsed.as_nanos()
            );
        } else {
            crate::log!(
                "path calc time: {}({})",
                elapsed.as_millis(),
                elapsed.as_nanos()
            );
            if crate::bench() {
                writeln!(crate::bench::nav(), "{}", elapsed.as_millis()).unwrap();
            }
        }
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
    pub fn open_door_flood(&mut self, door: Vector) {
        match self.get_open_door_flood_start(door) {
            ShouldFloodDoor::Yes(pos) => {
                let start = std::time::Instant::now();
                let index = self.to_index(door);
                self.reachable[index] = true;
                let index = self.to_index(pos);
                self.reachable[index] = true;
                let mut seen = HashSet::new();
                let mut next = VecDeque::new();
                seen.insert(pos);
                next.push_front(pos);
                while let Some(pos) = next.pop_back() {
                    let index = self.to_index(pos);
                    self.reachable[index] = true;
                    let adj = self.get_adjacent(pos, None, false);
                    macro_rules! helper {
                        ($name: ident) => {
                            if adj.$name && !seen.contains(&pos.$name()) {
                                seen.insert(pos.$name());
                                next.push_front(pos.$name());
                            }
                        };
                    }
                    helper!(up);
                    helper!(down);
                    helper!(left);
                    helper!(right);
                }
                if crate::bench() {
                    let elapsed = start.elapsed();
                    writeln!(crate::bench::open_flood(), "{}", elapsed.as_millis()).unwrap();
                }
            }
            ShouldFloodDoor::NoNeed => {
                // Even if we don't need to flood the new area, we still need to mark the door as
                // reachable
                let index = self.to_index(door);
                self.reachable[index] = true;
            }
            ShouldFloodDoor::No => {}
        }
    }
    fn get_open_door_flood_start(&self, door: Vector) -> ShouldFloodDoor {
        let adj = self.get_adjacent(door, None, false);
        if adj.up && self.is_reachable(door.down()) {
            if self.is_reachable(door.up()) {
                return ShouldFloodDoor::NoNeed;
            }
            ShouldFloodDoor::Yes(door.up())
        } else if adj.down && self.is_reachable(door.up()) {
            if self.is_reachable(door.down()) {
                return ShouldFloodDoor::NoNeed;
            }
            ShouldFloodDoor::Yes(door.down())
        } else if adj.left && self.is_reachable(door.right()) {
            if self.is_reachable(door.left()) {
                return ShouldFloodDoor::NoNeed;
            }
            ShouldFloodDoor::Yes(door.left())
        } else if adj.right && self.is_reachable(door.left()) {
            if self.is_reachable(door.right()) {
                return ShouldFloodDoor::NoNeed;
            }
            ShouldFloodDoor::Yes(door.right())
        } else {
            ShouldFloodDoor::No
        }
    }
    pub fn flood(&mut self, player: Vector) {
        let start = std::time::Instant::now();
        for reachable in self.reachable.iter_mut() {
            *reachable = false;
        }
        let mut to_visit = VecDeque::new();
        let mut seen = HashSet::new();
        to_visit.push_front(player);
        seen.insert(player);
        while let Some(pos) = to_visit.pop_back() {
            let index = self.to_index(pos);
            self.reachable[index] = true;
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
        );
        if crate::bench() {
            writeln!(crate::bench::flood(), "{}", elapsed.as_millis()).unwrap();
        }
    }
    pub fn move_and_think(
        &mut self,
        player: &mut Player,
        enemy: Arc<RwLock<Enemy>>,
        bounds: Range<Vector>,
        time: &mut std::time::Duration,
        do_delay: bool,
    ) {
        if self.move_enemy(player, enemy.clone())
            && self.is_visible(
                enemy.try_read().unwrap().pos,
                bounds.clone(),
                player.effects.full_vis.is_active(),
            )
        {
            self.smart_render(player);
            if do_delay {
                std::thread::sleep(crate::DELAY);
            }
        }
        if self.is_reachable(enemy.try_read().unwrap().pos) && enemy.try_read().unwrap().active {
            enemy.try_read().unwrap().alert_nearby(
                Arc::as_ptr(&enemy).addr(),
                self,
                crate::random() as usize & 7,
            );
        }
        let think = Enemy::think(enemy.clone(), self, player, time);
        if think
            && self.is_visible(
                enemy.try_read().unwrap().pos,
                bounds.clone(),
                player.effects.full_vis.is_active(),
            )
        {
            self.smart_render(player);
            if do_delay {
                std::thread::sleep(crate::DELAY);
            }
        }
    }
    pub fn move_enemy(&mut self, player: &mut Player, arc: Arc<RwLock<Enemy>>) -> bool {
        let mut enemy = arc.try_write().unwrap();
        let addr = Arc::as_ptr(&arc).addr();
        if !enemy.active || !self.is_reachable(enemy.pos) || enemy.attacking || enemy.is_stunned() {
            return false;
        }
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
        for boss in self.bosses.iter_mut() {
            if let Some(sibling) = boss.sibling.upgrade() {
                if Arc::ptr_eq(&sibling, &arc) {
                    boss.last_pos = enemy.pos;
                }
            }
        }
        true
    }
    pub fn contains_enemy(&self, pos: Vector, addr: Option<usize>) -> bool {
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
        for (pos, is_dead) in self
            .bosses
            .iter()
            .map(|boss| (boss.last_pos, boss.sibling.upgrade().is_none()))
            .collect::<Vec<(Vector, bool)>>()
            .into_iter()
        {
            if is_dead {
                self[pos] = Some(Piece::Exit(Exit::Shop));
            }
        }
    }
    pub fn update_spells(&mut self, player: &mut Player) {
        // Creating fake visual spells
        let mut specials = Vec::new();
        for circle in self.spells.iter() {
            let arc = Arc::new(Special::new(circle.pos, '∆', Some(*Style::new().purple())));
            self.specials.push(Arc::downgrade(&arc));
            specials.push(arc);
        }
        // Actually updating the circles
        let mut circles = std::mem::take(&mut self.spells);
        circles.retain_mut(|circle| circle.update(self, player));
        self.spells = circles;
        std::mem::drop(specials);
    }
    pub fn update_boss_pos(&mut self) {
        for boss in self.bosses.iter_mut() {
            if let Some(arc) = boss.sibling.upgrade() {
                boss.last_pos = arc.try_read().unwrap().pos;
            }
        }
    }
    pub fn show_path(&mut self, index: usize, player: Vector) {
        let mut pos = self.enemies[index].try_read().unwrap().pos;
        let mut index = self.to_index(pos);
        let mut specials = crate::ONE_TURN_SPECIALS.try_lock().unwrap();
        loop {
            if pos == player || self.backtraces[index].cost.is_none() {
                break;
            }
            pos += self.backtraces[index].from;
            index = self.to_index(pos);
            specials.push(self.add_special(Special::new(
                pos,
                ' ',
                Some(*Style::new().background_green()),
            )));
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
            // Bosses cannot be saved
            bosses: Vec::new(),
            visible: Vec::from_binary(binary)?,
            seen: Vec::from_binary(binary)?,
            turns_spent: usize::from_binary(binary)?,
            reachable: Vec::from_binary(binary)?,
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
        // skipping bosses because skipping enemies
        self.visible.to_binary(binary)?;
        self.seen.to_binary(binary)?;
        self.turns_spent.to_binary(binary)?;
        self.reachable.to_binary(binary)
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
    Sign(String),
}
impl Piece {
    fn render(&self, pos: Vector, board: &Board, player: &Player) -> (char, Option<Style>) {
        match self {
            Piece::Wall(_) => (Wall::render(pos, board), None),
            Piece::Door(door) => door.render(pos, board),
            Piece::Exit(_) => Exit::render(),
            Piece::Item(item) => item.render(player),
            Piece::Upgrade(upgrade) => upgrade.render(player),
            Piece::Sign(_) => ('S', None),
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
            Piece::Item(_) | Piece::Upgrade(_) | Piece::Sign(_) => false,
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
            Self::Sign(text) => write!(lock, "{text}").unwrap(),
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
            Piece::Sign(text) => write!(f, "sign with text: {text}"),
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
                    "sign" => Ok(Piece::Sign(args)),
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
            5 => Self::Sign(String::from_binary(binary)?),
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
            Self::Sign(text) => {
                5_u8.to_binary(binary)?;
                text.to_binary(binary)
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
            out.push(base + Direction::Up);
        }
        if self.down {
            out.push(base + Direction::Down);
        }
        if self.left {
            out.push(base + Direction::Left);
        }
        if self.right {
            out.push(base + Direction::Right);
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
enum ShouldFloodDoor {
    Yes(Vector), // Start point
    NoNeed,
    No,
}
#[derive(Clone)]
pub struct Boss {
    pub last_pos: Vector,
    pub sibling: Weak<RwLock<Enemy>>,
}
