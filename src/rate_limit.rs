const RATE_LIMIT: u8 = 10;

#[derive(Default)]
pub struct RateLimit {
    count: u8,
}

impl RateLimit {
    pub fn incr(&mut self) -> bool {
        if self.count > RATE_LIMIT {
            return false;
        }
        self.count += 1;
        true
    }

    pub fn reset(&mut self) {
        self.count = 0
    }
}
