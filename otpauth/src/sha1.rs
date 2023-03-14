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
    cipher.update(data);
    let stage1 = cipher.digest();

    let mut cipher = Sha1::new();
    cipher.update(&opad);
    cipher.update(&stage1.bytes());

    return cipher.digest().bytes();
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_with_a_key_of_less_than_64_bytes() {
        let digest = super::hmac(b"nosecret", b"\x00\x11\x22\x33\x44\x55\x66\x77\x88");
        assert_eq!(digest, *b"\x75\x14\xB9\x5D\x55\x69\x1D\x53\xB6\xC1\x59\xE7\xF0\x50\x2A\x8F\x7F\xA1\x40\x6E");
    }

    #[test]
    fn test_with_a_key_of_64_bytes() {
        let digest = super::hmac(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            b"\x00\x11\x22\x33\x44\x55\x66\x77\x88"
        );
        assert_eq!(digest, *b"\xFC\x53\x00\x13\x60\x40\x55\x51\x0B\xBC\x9D\x35\x77\x53\x9F\x37\x0F\xD9\x5E\x8C");
    }

    #[test]
    fn test_with_a_key_of_more_than_64_bytes() {
        let digest = super::hmac(
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=[]",
            b"\x00\x11\x22\x33\x44\x55\x66\x77\x88"
        );
        assert_eq!(digest, *b"\xA3\xA3\x54\x00\x3A\x63\x3C\x3D\x85\xEB\x80\x09\xF4\x9F\x5B\x79\xA8\x86\x5A\xE2");
    }
}
