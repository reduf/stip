use crate::sha1;
use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MOD_TABLE: [u32; 11] = [
    1,
    10,
    100,
    1000,
    10000,
    100000,
    1000000,
    10000000,
    100000000,
    1000000000,
    u32::MAX,
];

#[derive(Serialize)]
pub struct TotpToken {
    pub number: u32,
    created_at: SystemTime,
    not_after: SystemTime,
}

pub fn from_moving_factor(secret: &[u8], moving_factor: u64, digits: usize) -> u32 {
    let moving_factor_bytes = moving_factor.to_be_bytes();

    let mac = sha1::hmac(secret, &moving_factor_bytes);
    let offset = (mac[19] & 0xf) as usize;
    let number = (((mac[offset] & 0x7F) as u32) << 24)
        | (((mac[offset + 1] as u32) & 0xFF) << 16)
        | (((mac[offset + 2] as u32) & 0xFF) << 8)
        | ((mac[offset + 3] as u32) & 0xFF);

    let digits = std::cmp::min(digits, MOD_TABLE.len() - 1);
    return number % MOD_TABLE[digits];
}

pub fn from_seconds(secret: &[u8], timestamp: u64, digits: usize) -> u32 {
    return from_moving_factor(secret, timestamp / 30, digits);
}

pub fn from_now(secret: &[u8], digits: usize) -> TotpToken {
    let created_at = SystemTime::now();
    let seconds = created_at
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    let not_before = UNIX_EPOCH
        .checked_add(Duration::new((seconds / 30) * 30, 0))
        .expect("Couldn't create 'not_before'");
    let not_after = not_before
        .checked_add(Duration::new(30, 0))
        .expect("Couldn't create 'not_after'");
    let number = from_seconds(secret, seconds, digits);
    return TotpToken {
        number,
        created_at,
        not_after,
    };
}

pub fn progress() -> f32 {
    let window = 30u128 * 1000;
    let created_at = SystemTime::now();
    let seconds = created_at
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    let min = (seconds / window) * window;
    let offset = seconds - min;
    return (offset as f32) / 30000.0;
}

#[cfg(test)]
mod tests {
    #[test]
    fn works_with_specified_second() {
        let number = super::from_seconds(b"\x21\x22", 1678732967, 6);
        assert_eq!(number, 486091);
    }

    #[test]
    fn support_very_large_digits() {
        let number = super::from_seconds(b"\x21\x22", 1678732967, 32);
        assert_eq!(number, 783486091);
    }
}
