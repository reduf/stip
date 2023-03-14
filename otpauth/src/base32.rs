#[derive(Debug, Clone, Copy)]
pub enum ParseError {
    InvalidCharacter,
}

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

    // @cleanup: double check the initial capacity calculation.
    let mut result = Vec::with_capacity((input.len() / 8) * 5);

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

        if left >= 8 {
            result.push((buffer >> (left - 8)) as u8);
            buffer &= 0xFF;
            left -= 8;
        }
    }

    return Ok(result);
}

#[cfg(test)]
mod tests {
}
