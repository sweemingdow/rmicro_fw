use std::ops::Add;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[inline]
pub fn ts_mills() -> u64 {
    now_dur().as_millis() as u64
}

#[inline]
pub fn ts_secs() -> u64 {
    now_dur().as_secs()
}

#[inline]
fn now_dur() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}

pub fn plus(dur: Duration) -> u64 {
    SystemTime::now()
        .add(dur)
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[inline]
pub fn dur_from_days(days: u64) -> Duration {
    Duration::from_secs(days) * 24 * 60 * 60
}

#[inline]
pub fn dur_from_hours(hours: u64) -> Duration {
    Duration::from_secs(hours) * 60 * 60
}

#[inline]
pub fn dur_from_minutes(minutes: u64) -> Duration {
    Duration::from_secs(minutes) * 60
}
