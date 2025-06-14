use crate::{State, log};
use std::collections::BinaryHeap;

pub struct EventHandler {
    inner: BinaryHeap<Event>,
}
impl EventHandler {
    pub const fn new() -> EventHandler {
        EventHandler {
            inner: BinaryHeap::new(),
        }
    }
    pub fn handle(&mut self, state: &mut State) {
        loop {
            match self.inner.peek() {
                Some(event) => {
                    if event.trigger > state.turn {
                        return;
                    }
                }
                None => return,
            }
            (self.inner.pop().unwrap().action)(state)
        }
    }
    pub fn push(&mut self, trigger: usize, action: impl Fn(&mut State) + 'static) {
        self.inner.push(Event {
            trigger,
            action: Box::new(action),
        })
    }
}
pub struct Event {
    trigger: usize,
    action: Box<dyn Fn(&mut State)>,
}
impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.trigger.eq(&other.trigger)
    }
    fn ne(&self, other: &Self) -> bool {
        self.trigger.ne(&other.trigger)
    }
}
// Yes, I know this is cursed, but I need this to be a min heap
impl PartialOrd for Event {
    fn lt(&self, other: &Self) -> bool {
        self.trigger.gt(&other.trigger)
    }
    fn le(&self, other: &Self) -> bool {
        self.trigger.ge(&other.trigger)
    }
    fn gt(&self, other: &Self) -> bool {
        self.trigger.lt(&other.trigger)
    }
    fn ge(&self, other: &Self) -> bool {
        self.trigger.le(&other.trigger)
    }
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self < other {
            Some(std::cmp::Ordering::Less)
        } else if self > other {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}
impl Ord for Event {
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
    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
    {
        if self > max {
            max
        } else if self < min {
            min
        } else {
            self
        }
    }
}
impl Eq for Event {}
