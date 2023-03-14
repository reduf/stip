use sha1_smol::Sha1;

const SHA1_BLOCK_SIZE: usize = 64;

pub fn hmac(secret: &[u8], data: &[u8]) -> [u8; 20] {
    let mut key = [0u8; SHA1_BLOCK_SIZE];

    if SHA1_BLOCK_SIZE < secret.len() {
        let new_secret = Sha1::from(secret).digest();
        key[..20].copy_from_slice(&new_secret.bytes());
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }

    let mut ipad = [0x36u8; SHA1_BLOCK_SIZE];
    let mut opad = [0x5Cu8; SHA1_BLOCK_SIZE];

    for i in 0..SHA1_BLOCK_SIZE {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }

    let mut cipher = Sha1::new();
    cipher.update(&ipad);
    cipher.update(&data);
    let stage1 = cipher.digest();

    let mut cipher = Sha1::new();
    cipher.update(&opad);
    cipher.update(&stage1.bytes());

    return cipher.digest().bytes();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_with_a_key_of_less_than_64_bytes() {}

    #[test]
    fn test_with_a_key_of_64_bytes() {}

    #[test]
    fn test_with_a_key_of_more_than_64_bytes() {}
}
