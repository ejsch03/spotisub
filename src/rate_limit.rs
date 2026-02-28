use crate::prelude::*;

const RATE_LIMIT: u8 = 10;
const RATE_WINDOW: Duration = Duration::from_secs(60 * 60 * 24);

pub struct RateLimit {
    count: u8,
    window_start: Instant,
}

impl RateLimit {
    pub fn allow(&mut self) -> bool {
        if self.window_start.elapsed() >= RATE_WINDOW {
            self.count = 0;
            self.window_start = Instant::now();
        }
        self.count = self.count.saturating_add(1);
        self.count <= RATE_LIMIT
    }
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }
}
