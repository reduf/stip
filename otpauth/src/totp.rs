use crate::sha1;
use std::time::{SystemTime, UNIX_EPOCH};

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

pub fn from_moving_factor(secret: &[u8], moving_factor: u64, digits: usize) -> u32 {
    let moving_factor_bytes = moving_factor.to_be_bytes();

    let mac = sha1::hmac(secret, &moving_factor_bytes);
    let offset = (mac[19] & 0xf) as usize;
    let number = (((mac[offset + 0] & 0x7F) as u32) << 24)
        | (((mac[offset + 1] as u32) & 0xFF) << 16)
        | (((mac[offset + 2] as u32) & 0xFF) << 8)
        | ((mac[offset + 3] as u32) & 0xFF);

    return number % MOD_TABLE[(digits % MOD_TABLE.len())];
}

pub fn from_seconds(secret: &[u8], timestamp: u64, digits: usize) -> u32 {
    return from_moving_factor(secret, timestamp / 30, digits);
}

pub fn from_now(secret: &[u8], digits: usize) -> u32 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let seconds_since_epoch = since_the_epoch.as_secs();
    return from_seconds(secret, seconds_since_epoch, digits);
}

#[cfg(test)]
mod tests {
    #[test]
    fn works_with_specified_second() {
        let number = super::from_seconds(b"\x21\x22", 1678732967, 6);
        assert_eq!(number, 486091);
    }
}
