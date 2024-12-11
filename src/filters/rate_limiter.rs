use std::time::{Duration, Instant};

pub struct RateLimiter {
    pub max_messages: usize,
    pub time_window: Duration,
}

impl RateLimiter {
    pub fn new(max_messages: usize, window_seconds: u64) -> Self {
        Self {
            max_messages,
            time_window: Duration::from_secs(window_seconds),
        }
    }

    pub fn is_rate_limited(&self, message_count: usize, last_message_time: Instant) -> bool {
        let now = Instant::now();
        let time_diff = now.duration_since(last_message_time);

        time_diff <= self.time_window && message_count >= self.max_messages
    }
}
