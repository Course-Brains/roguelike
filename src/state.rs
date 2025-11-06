use crate::*;
pub struct State {
    pub player: Player,
    pub board: Board,
    pub turn: usize,
    pub next_map: std::thread::JoinHandle<Board>,
    pub next_map_settings: MapGenSettings,
    pub next_shop: std::thread::JoinHandle<Board>,
    pub shop_sender: std::sync::mpsc::Sender<Vec<upgrades::UpgradeType>>,
    // debugging
    pub nav_stepthrough: bool,
    pub nav_stepthrough_index: Option<usize>,
    pub show_nav: bool,
    pub show_nav_index: Option<usize>,
}
impl State {
    pub fn new(initial_board: InitialBoard) -> State {
        State {
            player: Player::new(Vector::new(1, 1)),
            board: match initial_board {
                InitialBoard::Normal => generate(MapGenSettings::new(
                    151,
                    151,
                    State::level_0_budget(),
                    1,
                    State::level_0_highest_tier(),
                )),
                InitialBoard::Empty => std::thread::spawn(Board::new_empty),
                InitialBoard::Tutorial => std::thread::spawn(Board::new_tutorial),
            }
            .join()
            .unwrap(),
            turn: 0,
            next_map: std::thread::spawn(|| Board::new(10, 10)),
            next_map_settings: MapGenSettings::new(
                501,
                501,
                State::level_1_budget(),
                3,
                State::level_1_highest_tier(),
            ),
            next_shop: std::thread::spawn(Board::new_shop),
            shop_sender: SHOP_AVAILABILITY.try_lock().unwrap().0.clone(),
            nav_stepthrough: false,
            nav_stepthrough_index: None,
            show_nav: false,
            show_nav_index: None,
        }
    }
    pub fn level_0_budget() -> usize {
        match SETTINGS.difficulty() {
            Difficulty::Normal => 75,
            Difficulty::Easy => 50,
            Difficulty::Hard => 500,
        }
    }
    pub fn level_1_budget() -> usize {
        match SETTINGS.difficulty() {
            Difficulty::Normal => 1500,
            Difficulty::Easy => 1000,
            Difficulty::Hard => 5000,
        }
    }
    pub fn level_0_highest_tier() -> Option<usize> {
        (SETTINGS.difficulty() <= Difficulty::Normal).then_some(2)
    }
    pub fn level_1_highest_tier() -> Option<usize> {
        State::level_0_highest_tier()
    }
    // returns if an enemy was hit
    pub fn attack_enemy(
        &mut self,
        pos: Vector,
        redrawable: bool,
        dashstun: bool,
        walking: bool,
    ) -> bool {
        for (index, enemy) in self.board.enemies.iter_mut().enumerate() {
            if enemy.try_read().unwrap().pos == pos {
                if dashstun {
                    enemy.try_write().unwrap().apply_dashstun()
                }
                if enemy
                    .try_write()
                    .unwrap()
                    .attacked(self.player.get_damage())
                {
                    let binding = self.board.enemies.swap_remove(index);
                    let killed = binding.try_read().unwrap();
                    self.player.on_kill(&killed);
                    killed.variant.on_death(&mut self.player);
                    if redrawable {
                        self.render()
                    }
                }
                stats().damage_dealt += self.player.get_damage();
                stats().attacks_done += 1;
                if walking {
                    stats().enemies_hit_by_walking += 1;
                }
                return true;
            }
        }
        false
    }
    pub fn is_on_board(&self, start: Vector, direction: Direction) -> bool {
        match direction {
            Direction::Up => {
                if start.y == 0 {
                    return false;
                }
            }
            Direction::Down => {
                if start.y == self.board.y - 1 {
                    return false;
                }
            }
            Direction::Left => {
                if start.x == 0 {
                    return false;
                }
            }
            Direction::Right => {
                if start.x == self.board.x - 1 {
                    return false;
                }
            }
        }
        true
    }
    pub fn is_valid_move(&self, direction: Direction) -> bool {
        if self.is_on_board(self.player.pos, direction) {
            return !self.board.has_collision(self.player.pos + direction);
        }
        false
    }
    pub fn think(&mut self, time: &mut std::time::Duration) {
        if self.player.effects.regen.is_active() {
            self.player.heal(2)
        }
        if self.player.effects.poison.is_active() {
            let _ = self.player.attacked(1, "poison".to_string(), None);
        }
        self.board.generate_nav_data(
            self.player.pos,
            self.nav_stepthrough,
            self.nav_stepthrough_index,
            &mut self.player,
        );
        let bounds = self.board.get_render_bounds(&self.player);
        let visible = self
            .board
            .get_visible_indexes(bounds.clone(), self.player.effects.full_vis.is_active());
        for (index, enemy) in self.board.enemies.clone().iter().enumerate() {
            self.board.move_and_think(
                &mut self.player,
                enemy.clone(),
                bounds.clone(),
                time,
                visible
                    .last()
                    .is_some_and(|last_index| *last_index == index),
            );
        }
        self.board.update_boss_pos();
        self.board.purge_dead(&mut self.player);
        if bench() {
            writeln!(bench::think(), "{}", time.as_millis()).unwrap();
        }
        self.board.update_spells(&mut self.player);
        self.board.place_exit();
    }
    pub fn render(&mut self) {
        let bounds = self.board.get_render_bounds(&self.player);
        self.board.smart_render(&mut self.player);
        self.draw_turn_level_and_money();
        self.player.reposition_cursor(
            self.board
                .has_background(self.player.selector, &self.player),
            bounds,
        );
    }
    pub fn draw_turn_level_and_money(&self) {
        crossterm::execute!(
            std::io::stdout(),
            crossterm::cursor::MoveTo(1, RENDER_Y as u16 * 2 + 4),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine)
        )
        .unwrap();
        print!(
            "turn: {}\x1b[30Glayer: {}\x1b[60Gmoney: {}",
            self.turn,
            layer(),
            self.player.get_money()
        );
    }
    pub fn increment(&mut self) {
        // Order of events:
        // decriment effects
        // enemies move and think (in that order)
        // last known boss positions are updated
        // dead are purged
        // spells are updated
        // exits are placed
        // turn increments
        // turn on map increments
        // show_nav one turn specials are placed
        // rendering
        // one turn specials reset
        // enemy damage taken flag reset
        let mut start = std::time::Instant::now();
        self.player.decriment_effects();
        let mut time = start.elapsed();
        self.think(&mut time);
        start = std::time::Instant::now();
        self.turn += 1;
        self.board.turns_spent += 1;
        if self.show_nav {
            match self.show_nav_index {
                Some(index) => self.board.show_path(index, self.player.pos),
                None => {
                    for index in 0..self.board.enemies.len() {
                        self.board.show_path(index, self.player.pos)
                    }
                }
            }
        }
        self.render();
        *ONE_TURN_SPECIALS.lock().unwrap() = Vec::new();
        self.board.reset_took_damage();
        time += start.elapsed();
        if bench() {
            writeln!(bench::total(), "{}", time.as_millis()).unwrap();
        }
    }
    pub fn load_next_map(&mut self) {
        generator::DO_DELAY.store(false, Ordering::SeqCst);
        stats().shop_turns.push(self.board.turns_spent);
        self.board = std::mem::replace(&mut self.next_map, std::thread::spawn(|| Board::new(1, 1)))
            .join()
            .unwrap();
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        let settings = MapGenSettings::new(501, 501, self.get_budget(), NUM_BOSSES, None);
        reset_bonuses();
        self.next_map = generate(settings);
        self.next_map_settings = settings;
        LAYER.fetch_add(1, RELAXED);
        self.player.pos = Vector::new(1, 1);
        self.player.selector = Vector::new(1, 1);
        self.board.flood(self.player.pos);
        self.player.memory = None;
        self.shop_sender
            .send(self.player.upgrades.get_available())
            .unwrap();
        self.render();
    }
    pub fn get_budget(&self) -> usize {
        let mut budget = (self.turn / BUDGET_DIVISOR) + (layer() * BUDGET_PER_LAYER);
        if SETTINGS.difficulty() <= Difficulty::Easy {
            budget /= 2;
        } else if SETTINGS.difficulty() >= Difficulty::Hard {
            budget *= 4;
        }
        budget
    }
    pub fn load_shop(&mut self) {
        stats().level_turns.push(self.board.turns_spent);
        let bonus_kill_all = self.board.enemies.is_empty();
        self.board = std::mem::replace(&mut self.next_shop, std::thread::spawn(Board::new_shop))
            .join()
            .unwrap();
        self.player.pos = Vector::new(44, 14);
        self.player.selector = Vector::new(44, 14);

        // bonuses
        if BONUS_NO_WASTE.load(RELAXED) {
            self.board[Board::BONUS_NO_WASTE] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoWaste)),
            ));
        }
        if BONUS_NO_DAMAGE.load(Ordering::Relaxed) {
            self.board[Board::BONUS_NO_DAMAGE] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoDamage)),
            ));
        }
        if bonus_kill_all {
            self.board[Board::BONUS_KILL_ALL] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusKillAll)),
            ));
        }
        if BONUS_NO_ENERGY.load(RELAXED) {
            self.board[Board::BONUS_NO_ENERGY] = Some(board::Piece::Upgrade(
                pieces::upgrade::Upgrade::new(Some(upgrades::UpgradeType::BonusNoEnergy)),
            ));
        }

        stats().shop_money.push(self.player.get_money());
        self.player.memory = None;
        if SETTINGS.difficulty() >= Difficulty::Normal
            && self.player.energy > 1
            && random::random4() == 1
        {
            set_feedback(get(31));
            self.player.energy /= 2;
            bell(Some(&mut std::io::stdout()));
        }
        self.render();
    }
    pub fn reposition_cursor(&mut self) {
        self.player.reposition_cursor(
            self.board
                .has_background(self.player.selector, &self.player),
            self.board.get_render_bounds(&self.player),
        );
    }
    pub fn is_visible(&self, pos: Vector) -> bool {
        self.board.is_visible(
            pos,
            self.board.get_render_bounds(&self.player),
            self.player.effects.full_vis.is_active(),
        )
    }
    pub fn open_door(&mut self, pos: Vector, walking: bool) {
        if let Some(Piece::Door(door)) = &mut self.board[pos] {
            // Closing the door
            if door.open {
                door.open = false;
                stats().doors_closed += 1;

                let reachable_bosses: Vec<Vector> = self
                    .board
                    .bosses
                    .iter()
                    .filter(|boss| boss.sibling.upgrade().is_some())
                    .map(|boss| boss.last_pos)
                    .collect::<Vec<Vector>>()
                    .iter()
                    .filter(|pos| self.board.is_reachable(**pos))
                    .copied()
                    .collect();
                if !reachable_bosses.is_empty() {
                    self.board.flood(self.player.pos);
                    if reachable_bosses
                        .iter()
                        .any(|pos| self.board.is_reachable(*pos))
                    {
                        stats().cowardice += 1;
                    }
                    return;
                }
                // we don't need to explicitly set the closed door as unreachable because
                // the flood will do that for us
                re_flood();
            // Opening the door
            } else {
                stats().doors_opened += 1;
                if walking {
                    stats().doors_opened_by_walking += 1;
                }
                self.board.open_door_flood(pos);
                self.board[pos] = Some(Piece::Door(pieces::door::Door { open: true }));
            }
            self.increment();
        }
    }
}
impl FromBinary for State {
    fn from_binary(binary: &mut dyn std::io::Read) -> Result<Self, std::io::Error>
    where
        Self: Sized,
    {
        if Version::from_binary(binary)? != SAVE_VERSION {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid save format".to_string(),
            ));
        }
        CHEATS.store(bool::from_binary(binary)?, Ordering::Relaxed);
        let difficulty = settings::Difficulty::from_binary(binary)?;
        if difficulty != SETTINGS.difficulty() {
            println!("Don't change the difficulty mid run, go set it back to {difficulty}");
            bell(None);
            return Err(std::io::Error::other(""));
        }
        LAYER.store(usize::from_binary(binary)?, RELAXED);
        generator::DO_DELAY.store(true, Ordering::SeqCst);
        let settings = MapGenSettings::from_binary(binary)?;
        Ok(State {
            player: Player::from_binary(binary)?,
            board: Board::from_binary(binary)?,
            turn: usize::from_binary(binary)?,
            next_map: generate(settings),
            next_map_settings: settings,
            next_shop: std::thread::spawn(Board::new_shop),
            shop_sender: SHOP_AVAILABILITY.lock().unwrap().0.clone(),
            nav_stepthrough: bool::from_binary(binary)?,
            nav_stepthrough_index: Option::from_binary(binary)?,
            show_nav: bool::from_binary(binary)?,
            show_nav_index: Option::from_binary(binary)?,
        })
    }
}
impl ToBinary for State {
    fn to_binary(&self, binary: &mut dyn Write) -> Result<(), std::io::Error> {
        SAVE_VERSION.to_binary(binary)?;
        CHEATS.load(Ordering::Relaxed).to_binary(binary)?;
        SETTINGS.difficulty().to_binary(binary)?;
        LAYER.load(RELAXED).to_binary(binary)?;
        self.next_map_settings.to_binary(binary)?;
        self.player.to_binary(binary)?;
        self.board.to_binary(binary)?;
        self.turn.to_binary(binary)?;
        // Cannot save shop_sender
        self.nav_stepthrough.to_binary(binary)?;
        self.nav_stepthrough_index.as_ref().to_binary(binary)?;
        self.show_nav.to_binary(binary)?;
        self.show_nav_index.as_ref().to_binary(binary)
    }
}
