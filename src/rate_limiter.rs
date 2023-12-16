use std::sync::{Arc, Mutex};
/// The goal here is to allow us to rate limit ourselves nicely.
/// We want to be able to
///     - immediately allow requests if they will not violate the rate limit
///     - block until we can make another request
use std::thread::sleep;
use std::time::{Duration, Instant};

use queues::{CircularBuffer, IsQueue};

#[derive(Clone, Debug)]
pub struct RateLimiter {
    duration: Duration,
    prev_requests: Arc<Mutex<CircularBuffer<Instant>>>,
}

impl RateLimiter {
    // TODO want to add a "get burst capacity" option, which returns the
    // number of requests that are likely to succeed immediately

    pub fn new(num_requests: usize, over_duration: Duration) -> Self {
        Self {
            duration: over_duration,
            prev_requests: Arc::new(Mutex::new(CircularBuffer::<Instant>::new(num_requests))),
        }
    }

    /// Returns the duration until we can make a request
    fn time_until_available(&self) -> Duration {
        let prev_reqs = self.prev_requests.lock().expect("attempting lock");

        if prev_reqs.size() < prev_reqs.capacity() {
            return Duration::new(0, 0);
        }

        let dt = Instant::now()
            .checked_duration_since(prev_reqs.peek().expect("guaranteed to not be empty"))
            .unwrap();

        if dt < self.duration {
            self.duration - dt
        } else {
            Duration::new(0, 0)
        }
    }

    /// Get the "average pace" for the rate limit - that is, we should be able to
    /// make one request each "average pace" duration indefinitely without violating
    /// the rate limit
    fn get_average_pace(&self) -> Duration {
        let prev_reqs = self.prev_requests.lock().expect("attempting lock");

        self.duration / prev_reqs.capacity() as u32
    }

    /// Returns true if we can make a request, otherwise, false
    pub fn attempt(&self) -> bool {
        self.time_until_available() == Duration::new(0, 0)
    }

    /// Returns true if we can make a request,
    /// and if so, commits. Otherwise, false
    pub fn attempt_commit(&mut self) -> bool {
        let is_ok = self.attempt();

        if is_ok {
            let now = Instant::now();
            self.prev_requests
                .lock()
                .expect("attempting lock")
                .add(now)
                .unwrap();
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

        let time_until_available = self.time_until_available();

        if time_until_available > timeout {
            return false;
        }

        sleep(time_until_available);

        // Now we should be able to make a request. If we *cant*
        // make the request, something has gone terribly wrong and
        // we should panic (I believe)
        assert!(self.attempt_commit(), "should have succeeded");

        true
    }

    /// Will attempt to block for the "average pace" (duration / num reqs).
    /// If we have to wait for longer than the average pace, we block then
    /// commit with the timeout.
    pub fn block_for_average_pace_then_commit(&mut self, timeout: Duration) -> bool {
        let avg_pace = self.get_average_pace();

        if self.attempt() {
            // since we can make an attempt, sleep for avg pace and then commit
            sleep(avg_pace);
            assert!(self.attempt_commit(), "should have succeeded");
            true
        } else {
            // if we can't make an attempt, we need to block until
            let time_until_free = self.time_until_available();
            if time_until_free < avg_pace {
                sleep(avg_pace);
                assert!(self.attempt_commit(), "should have succeeded");
                true
            } else {
                self.block_then_commit(timeout)
            }
        }
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

        assert!(rl.attempt_commit());
        assert!(rl.attempt_commit());
        assert!(rl.attempt_commit());
        assert!(!rl.attempt_commit());

        sleep(Duration::from_millis(110));

        assert!(rl.attempt_commit());
        assert!(rl.attempt_commit());
        assert!(rl.attempt_commit());
        assert!(!rl.attempt_commit());
    }

    #[test]
    fn test_rate_limiter_2() {
        // attempt doesn't change the state
        let rl = RateLimiter::new(1, Duration::from_millis(100));

        assert!(rl.attempt());
        assert!(rl.attempt());
        assert!(rl.attempt());
        assert!(rl.attempt());
    }

    #[test]
    fn test_rate_limiter_3() {
        let mut rl = RateLimiter::new(1, Duration::from_millis(100));

        // should successfully commit
        assert!(rl.attempt_commit());

        // should fail to commit since we *just* added it
        assert!(!rl.attempt());

        // should succeed since we have to block for less time than the timeout
        // (therefore we don't timeout, therefore it's true)
        assert!(rl.block_then_commit(Duration::from_millis(110)));
    }

    #[test]
    fn test_rate_limiter_4() {
        // attempt doesn't change the state
        let mut rl = RateLimiter::new(1, Duration::from_millis(100));

        assert!(rl.attempt_commit());
        assert!(!rl.attempt());
        assert!(!rl.block_then_commit(Duration::from_millis(1)));
    }

    #[test]
    fn test_rate_limiter_5() {
        // attempt doesn't change the state
        let mut rl = RateLimiter::new(1, Duration::from_millis(100));

        assert!(rl.time_until_available() == Duration::new(0, 0));

        rl.attempt_commit();

        assert!(rl.time_until_available() > Duration::new(0, 0));
    }

    #[test]
    fn test_rate_limiter_6() {
        let mut rl = RateLimiter::new(10, Duration::from_millis(100));

        // fill up the rl real quick
        for _ in 0..10 {
            assert!(rl.attempt_commit());
        }

        // If the timeout is less than the average pace, we should timeout
        assert!(!rl.block_for_average_pace_then_commit(Duration::from_millis(1)));

        // But if the timeout is greater than the average pace, we should wait for the avg pace.
        // So first, put a fresh request in
        rl.block_then_commit(Duration::from_millis(110));
        // and now we should wait at *least* 10 ms
        let start = Instant::now();
        assert!(rl.block_for_average_pace_then_commit(Duration::from_millis(1000)));
        let elapsed = start.elapsed();
        // TODO this will be sensitive to the speed of the machine, but I think a ms is really long
        assert!(
            elapsed > Duration::from_millis(10),
            "elapsed: {:?}",
            elapsed
        );
        assert!(
            elapsed < Duration::from_millis(15),
            "elapsed: {:?}",
            elapsed
        );
    }
}
