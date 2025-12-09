// Convenience constant
pub const RELAXED: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;

// The number that the turn count is divided by to get the budget
pub const BUDGET_DIVISOR: usize = 5;

// The number of bosses in each level starting at the third level
pub const NUM_BOSSES: usize = 5;

// The budget given per layer (layer * this)
pub const BUDGET_PER_LAYER: usize = 100;

// The distance from the center of the render area to the horizontal walls
pub const RENDER_X: usize = 45;

// The distance from the center of the render area to the vertical edges
pub const RENDER_Y: usize = 15;

// Delay between moves/applicable things
pub const DELAY: std::time::Duration = std::time::Duration::from_millis(100);

// Delay between subtick animaion frames
pub const PROJ_DELAY: std::time::Duration = std::time::Duration::from_millis(25);
pub fn proj_delay() {
    std::thread::sleep(PROJ_DELAY);
}

// The type used for file versions, don't change this, don't be dumb
pub type Version = u32;

// the path to the file used for saving and loading
pub const PATH: &str = "save";

// The path to the file of stats for previous runs
pub const STAT_PATH: &str = "stats";

// The minimum amount of time a turn has to take in order to purge the input queue.
// If you find that it is purging inputs when you don't want it to, increase this. If you find that
// it is not purging input you want it to, decrease this.
pub const INPUT_QUEUE_PURGE_THRESHOLD: std::time::Duration = std::time::Duration::from_millis(300);

// The amount of time given to the input thread to send all inputs given during the turn that the
// input queue is being purged for. This is a bit complicated and I do not recommend changing it
// generally. However, if the input queue purge is not purging all inputs, then try increasing
// this.
pub const INPUT_QUEUE_PURGE_TIME: std::time::Duration = std::time::Duration::from_millis(50);
