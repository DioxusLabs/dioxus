use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Instant(Duration);

impl Instant {
    pub fn new(ts: f64) -> Self {
        Self(Duration::from_millis(ts as u64))
    }
}

impl From<Instant> for f64 {
    fn from(instant: Instant) -> Self {
        instant.0.as_secs_f64()
    }
}
