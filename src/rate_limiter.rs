/// The goal here is to allow us to rate limit ourselves nicely.
/// We want to be able to
///     - immediately allow requests if they will not violate the rate limit
///     - block until we can make another request

use std::thread::sleep;
use std::time::{Duration, Instant};

use queues::{IsQueue,CircularBuffer};

pub struct RateLimiter {
    duration: Duration,
    prev_requests: CircularBuffer::<Instant>,
}

/// Reaaaaly basic rate limiter
/// Constant time peak, push
#[allow(dead_code)]
impl RateLimiter {
    // TODO want to add a "burst capacity" option, which returns the
    // number of requests that are likely to succeed immediately

    pub fn new(num_requests: usize, over_duration: Duration) -> Self {
        Self {
            duration: over_duration,
            prev_requests: CircularBuffer::<Instant>::new(num_requests),
        }
    }

    /// Returns true if we can make a request, otherwise, false
    pub fn attempt(&self) -> bool {
        if self.prev_requests.size() < self.prev_requests.capacity() {
            return true;
        }

        let next_el_for_removal =
            self.prev_requests
                .peek()
                .expect("queue was empty, should be impossible!");

        let dt = Instant::now()
            .checked_duration_since(next_el_for_removal)
            .unwrap();

        if dt >= self.duration {
            true
        } else {
            false
        }
    }

    /// Returns true if we can make a request,
    /// and if so, commits. Otherwise, false
    pub fn attempt_commit(&mut self) -> bool {
        let is_ok = self.attempt();

        if is_ok {
            let now = Instant::now();
            self.prev_requests.add(now).unwrap();
        }

        is_ok
    }

    /// Blocks until we can make a request, unless we hit timeout,
    /// in which case we return false. If we don't hit the timeout,
    /// we return true.
    pub fn block_then_commit(&mut self, timeout: Duration) -> bool {
        if self.attempt_commit() {
            return true;
        }

        let next_el_for_removal = self.prev_requests
            .peek()
            .expect("queue was empty, should be impossible!");

        let dt = Instant::now()
            .checked_duration_since(next_el_for_removal)
            .unwrap();

        if self.duration - dt > timeout {
            return false;
        }

        sleep(self.duration - dt);

        // Now we should be able to make a request. If we *cant*
        // make the request, something has gone terribly wrong and
        // we should panic (I believe)
        assert!(self.attempt_commit(), "should have succeeded");

        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_1() {
        // attempt_commit should return true when it can commit,
        // and false when it can't
        let mut rl = RateLimiter::new(3, Duration::from_millis(100));

        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), false);

        sleep(Duration::from_millis(110));

        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt_commit(), false);
    }

    #[test]
    fn test_rate_limiter_2() {
        // attempt doesn't change the state
        let rl = RateLimiter::new(1, Duration::from_millis(100));

        assert_eq!(rl.attempt(), true);
        assert_eq!(rl.attempt(), true);
        assert_eq!(rl.attempt(), true);
        assert_eq!(rl.attempt(), true);
    }

    #[test]
    fn test_rate_limiter_3() {
        let mut rl = RateLimiter::new(1, Duration::from_millis(100));

        // should successfully commit
        assert_eq!(rl.attempt_commit(), true);

        // should fail to commit since we *just* added it
        assert_eq!(rl.attempt(), false);

        // should succeed since we have to block for less time than the timeout
        // (therefore we don't timeout, therefore it's true)
        assert_eq!(rl.block_then_commit(Duration::from_millis(110)), true);
    }

    #[test]
    fn test_rate_limiter_4() {
        // attempt doesn't change the state
        let mut rl = RateLimiter::new(1, Duration::from_millis(100));

        assert_eq!(rl.attempt_commit(), true);
        assert_eq!(rl.attempt(), false);
        assert_eq!(rl.block_then_commit(Duration::from_millis(1)), false);
    }
}
