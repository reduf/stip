#[derive(Debug, Clone, Copy)]
pub enum ParseError {
    InvalidCharacter,
}

#[allow(clippy::identity_op)]
pub fn b32decode(input: &[u8]) -> Result<Vec<u8>, ParseError> {
    fn value(b32char: u8) -> Result<u8, ParseError> {
        return match b32char {
            b'A'..=b'Z' => Ok((b32char - b'A') + 0),
            b'a'..=b'z' => Ok((b32char - b'a') + 0),
            b'2'..=b'7' => Ok((b32char - b'2') + 26),
            _ => Err(ParseError::InvalidCharacter),
        };
    }

    let mut buffer = 0u32;
    let mut left = 0u32;

    let mut result = Vec::with_capacity(((input.len() + 4) / 5) * 8);

    let mut it = input.iter();
    while let Some(char) = it.next().copied() {
        if char == b'=' {
            while let Some(char) = it.next().copied() {
                if char != b'=' {
                    return Err(ParseError::InvalidCharacter);
                }
            }

            break;
        }

        buffer = (buffer << 5) | value(char)? as u32;
        left += 5;

        if 8 <= left {
            result.push((buffer >> (left - 8)) as u8);
            buffer &= 0xFF;
            left -= 8;
        }
    }

    return Ok(result);
}

pub fn b32encode(input: &[u8]) -> String {
    fn value(bits: usize) -> char {
        const BASE32: &'static [u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        return BASE32[bits & 0x1F] as char;
    }

    fn pad(buffer: &mut String, count: usize) {
        for _ in 0..count {
            buffer.push('=');
        }
    }

    let mut buffer = 0u64;
    let mut left = 0u64;

    let mut result = String::with_capacity(((input.len() * 8) + (4)) / 5);
    for byte in input.iter().copied() {
        buffer = (buffer << 8) | (byte as u64);
        left += 8;

        if left == 40 {
            result.push(value(((buffer >> 35) & 0x1F) as usize));
            result.push(value(((buffer >> 30) & 0x1F) as usize));
            result.push(value(((buffer >> 25) & 0x1F) as usize));
            result.push(value(((buffer >> 20) & 0x1F) as usize));
            result.push(value(((buffer >> 15) & 0x1F) as usize));
            result.push(value(((buffer >> 10) & 0x1F) as usize));
            result.push(value(((buffer >>  5) & 0x1F) as usize));
            result.push(value(((buffer >>  0) & 0x1F) as usize));

            left = 0;
            buffer = 0;
        }
    }

    match left {
        0 => (),
        8 => {
            buffer = buffer << 2;
            result.push(value(((buffer >> 5) & 0x1F) as usize));
            result.push(value(((buffer >> 0) & 0x1F) as usize));
            pad(&mut result, 6);
        },
        16 => {
            buffer = buffer << 4;
            result.push(value(((buffer >> 15) & 0x1F) as usize));
            result.push(value(((buffer >> 10) & 0x1F) as usize));
            result.push(value(((buffer >> 5)  & 0x1F) as usize));
            result.push(value(((buffer >> 0)  & 0x1F) as usize));
            pad(&mut result, 4);
        },
        24 => {
            buffer = buffer << 1;
            result.push(value(((buffer >> 20) & 0x1F) as usize));
            result.push(value(((buffer >> 15) & 0x1F) as usize));
            result.push(value(((buffer >> 10) & 0x1F) as usize));
            result.push(value(((buffer >> 5)  & 0x1F) as usize));
            result.push(value(((buffer >> 0)  & 0x1F) as usize));
            pad(&mut result, 3);

        },
        32 => {
            buffer = buffer << 3;
            result.push(value(((buffer >> 30) & 0x1F) as usize));
            result.push(value(((buffer >> 25) & 0x1F) as usize));
            result.push(value(((buffer >> 20) & 0x1F) as usize));
            result.push(value(((buffer >> 15) & 0x1F) as usize));
            result.push(value(((buffer >> 10) & 0x1F) as usize));
            result.push(value(((buffer >> 5)  & 0x1F) as usize));
            result.push(value(((buffer >> 0)  & 0x1F) as usize));
            pad(&mut result, 1);
        },
        _ => unreachable!(),
    }

    return result;
}

#[cfg(test)]
mod tests {
    static GOOD_BASE32_TESTS: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"OM======", b"\x73"),
        (b"GQZQ====", b"\x34\x33"),
        (b"fe3ue===", b"\x29\x37\x42"),
        (b"2RAJ4KA=", b"\xD4\x40\x9E\x28"),
        (b"irvzqone", b"\x44\x6B\x98\x39\xA4"),
        (b"TOX2D7T5WIZXWBO62FLMRLD2PPMXZ5YBL7BC2===", b"\x9B\xAF\xA1\xFE\x7D\xB2\x33\x7B\x05\xDE\xD1\x56\xC8\xAC\x7A\x7B\xD9\x7C\xF7\x01\x5F\xC2\x2D"),
        (b"zowrzjxqh2pfzc54dc4baviexlvxdtito4m4igi=", b"\xCB\xAD\x1C\xA6\xF0\x3E\x9E\x5C\x8B\xBC\x18\xB8\x10\x55\x04\xBA\xEB\x71\xCD\x13\x77\x19\xC4\x19"),
        (b"5FB2PALLET2NFGZ72LCVC6PE6VBDF6PC2ZGPP7DD", b"\xE9\x43\xA7\x81\x6B\x24\xF4\xD2\x9B\x3F\xD2\xC5\x51\x79\xE4\xF5\x42\x32\xF9\xE2\xD6\x4C\xF7\xFC\x63"),
    ];

    static BAD_BASE32_TESTS: &[&[u8]] = &[b"a0======", b"a1======", b"a/======", b"0M=A===="];

    #[test]
    fn test_good_base32_strings() {
        for (input, expected) in GOOD_BASE32_TESTS {
            assert_eq!(super::b32decode(&input).unwrap().as_slice(), *expected);
        }
    }

    #[test]
    fn test_bad_base32_strings() {
        for input in BAD_BASE32_TESTS {
            super::b32decode(&input).unwrap_err();
        }
    }

    #[test]
    fn test_good_b32encode() {
        for (expected, input) in GOOD_BASE32_TESTS {
            assert_eq!(super::b32encode(&input).as_bytes(), expected.to_ascii_uppercase());
        }
    }
}
