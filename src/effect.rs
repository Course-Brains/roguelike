use crate::state::Entity;
use crate::state::State;
use abes_nice_things::require_debug;
use abes_nice_things::{FromBinary, ToBinary};
use anyhow::Result;

#[derive(Clone, Copy, Debug, Hash)]
pub struct Effect {
    pub name: &'static str,
    on_start: fn(&mut State, Entity),
    /// Whether or not the on_start should be reran on load or left alone
    run_on_start_on_load: bool,
    on_end: fn(&mut State, Entity),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectTracker {
    /// None is infinite time
    /// 0 is not active
    /// other is active for finite time
    inner: [Option<usize>; EFFECTS.len()],
}
impl ToBinary for EffectTracker {
    fn to_binary(&self, binary: &mut dyn std::io::prelude::Write) -> Result<()> {
        self.inner
            .each_ref()
            .map(|time| time.as_ref())
            .to_binary(binary)
    }
}
impl FromBinary for EffectTracker {
    fn from_binary(binary: &mut dyn std::io::prelude::Read) -> Result<Self> {
        Ok(EffectTracker {
            inner: <[Option<usize>; EFFECTS.len()]>::from_binary(binary)?,
        })
    }
}
impl Default for EffectTracker {
    fn default() -> Self {
        EffectTracker {
            inner: [Some(0); EFFECTS.len()],
        }
    }
}
impl EffectTracker {
    /// Decriments all effect timers and returns a list of the effects that ended
    pub fn decriment(&mut self) -> Vec<EffectID> {
        let mut finished = Vec::new();
        for (id, timer) in self
            .inner
            .iter_mut()
            .enumerate()
            .filter(|(_, time)| time.is_some())
            .map(|(index, time)| (index, time.as_mut().unwrap()))
        {
            if *timer == 1 {
                finished.push(EffectID::from_raw(id as u8))
            }
            *timer = timer.saturating_sub(1);
        }
        finished
    }
    pub fn run_on_ends(state: &mut State, entity: Entity, effects: Vec<EffectID>) {
        for effect in effects.into_iter() {
            (effect.get().on_end)(state, entity)
        }
    }
    pub fn has(&self, effect: EffectID) -> bool {
        self.inner[effect.to_raw() as usize].is_none_or(|time| time > 0)
    }
    /// Does NOT run on_start
    pub fn set_effect_time(&mut self, effect: EffectID, time: Option<usize>) {
        self.inner[effect.to_raw() as usize] = time
    }
    /// Does run on_start
    pub fn prompt_set_time(state: &mut State, entity: Entity, effect: EffectID) {
        let time = loop {
            let input = state.get_input("How many turns? ".to_string());
            break match input.as_str() {
                "cancel" | "c" | "quit" | "q" => return,
                "infinty" | "infinite" | "inf" | "i" => None,
                other => match other.parse::<usize>() {
                    Ok(time) => Some(time),
                    Err(error) => {
                        state.feedback = error.to_string();
                        state.render();
                        continue;
                    }
                },
            };
        };
        if time == Some(0) {
            EffectTracker::clear(state, entity, effect);
            return;
        }
        match entity {
            Entity::Player => {
                if !state.player.effect_tracker.has(effect) {
                    (effect.get().on_start)(state, entity);
                }
                state.player.effect_tracker.set_effect_time(effect, time);
            }
            Entity::Enemy(_) => abes_nice_things::require_debug!(todo!()),
        }
    }
    pub fn get(&self, effect: EffectID) -> Option<usize> {
        self.inner[effect.to_raw() as usize]
    }
    pub fn iter_effect_ids() -> impl Iterator<Item = EffectID> {
        (0..EFFECTS.len()).map(|index| EffectID::from_raw(index as u8))
    }
    pub fn clear(state: &mut State, entity: Entity, effect: EffectID) {
        match entity {
            Entity::Player => {
                if state.player.effect_tracker.has(effect) {
                    state.player.effect_tracker.inner[effect.to_raw() as usize] = Some(0);
                    (effect.get().on_end)(state, entity);
                }
            }
            Entity::Enemy(_) => require_debug!(todo!()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EffectID {
    Confusion = 0,
}
impl EffectID {
    pub fn from_raw(raw: u8) -> EffectID {
        if raw >= EFFECTS.len() as u8 {
            panic!("Tried to make invalid EffectID: ({raw})")
        }
        unsafe { std::mem::transmute(raw) }
    }
    fn to_raw(self) -> u8 {
        unsafe { std::mem::transmute(self) }
    }
    pub fn get(self) -> &'static Effect {
        &EFFECTS[self.to_raw() as usize]
    }
    pub fn needs_on_start_rerun_on_load(&self) -> bool {
        self.get().run_on_start_on_load
    }
    pub fn force_run_on_start(&self, state: &mut State, entity: Entity) {
        (self.get().on_start)(state, entity)
    }
}
pub static EFFECTS: &[Effect] = &[Effect {
    name: "Confusion",
    on_start: |_, entity| {
        if let Entity::Player = entity {
            print!("\x1b(0");
        }
    },
    run_on_start_on_load: true,
    on_end: |_, entity| {
        if let Entity::Player = entity {
            print!("\x1b(B");
        }
    },
}];
