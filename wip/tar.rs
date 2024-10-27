#[derive(Debug)]
pub struct ParseNumericError;

pub fn parse_numeric<const LEN: usize>(mut input: [u8; LEN]) -> Result<u64, ParseNumericError> {
  if input[0] & 0x80 != 0 {
    // Packed binary format.
    // Apart from the MSB of input[0], all bits before input[LEN-8] must be zeroes,
    // otherwise the logical value is too large to hold in a u64.
    input[0] &= 0x7F;
    for &byte in &input[..LEN - 8] {
      if byte != 0 {
        return Err(ParseNumericError);
      }
    }
    let input: &[u8] = &input[LEN - 8..];
    let input: &[u8; 8] = input.try_into().unwrap();
    Ok(u64::from_be_bytes(*input))
  } else {
    // ASCII octal format.
    let mut accum = 0;
    for byte in input {
      match byte {
        b'0'..=b'7' => {
          accum = accum * 8 + u64::from(byte - b'0');
        }
        b'\x00' | b' ' => {}
        _ => return Err(ParseNumericError),
      }
    }
    Ok(accum)
  }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Header<'a>(pub &'a [u8; 512]);

impl<'a> Header<'a> {
  pub fn content_len(self) -> Result<u64, ParseNumericError> {
    let bytes: &[u8; 12] = self.0[124..136].try_into().unwrap();
    parse_numeric(*bytes)
  }
  pub fn type_flag(self) -> u8 {
    self.0[156]
  }
  pub fn is_null(self) -> bool {
    self.type_flag() == 0 && self.0[0] == 0
  }
  pub fn is_prefix(self) -> bool {
    // x = metadata for the next file (PAX extension)
    // K = long linkname for the next file (GNU extension)
    // L = long name for the next file (GNU extension)
    matches!(self.type_flag(), b'x' | b'K' | b'L')
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_numeric_8_packed() {
    assert_eq!(
      parse_numeric([0x81, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).ok(),
      Some(0x0102030405060708),
    );
  }

  #[test]
  fn test_parse_numeric_12_packed() {
    assert_eq!(
      parse_numeric([0x80, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).ok(),
      Some(0x0102030405060708),
    );
  }

  #[test]
  fn test_parse_numeric_12_packed_overflow() {
    assert_eq!(
      parse_numeric([0x80, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09]).ok(),
      None,
    );
  }

  #[test]
  fn test_parse_numeric_12_ascii_small() {
    assert_eq!(parse_numeric(*b"00000000017\x00").ok(), Some(15),)
  }

  #[test]
  fn test_parse_numeric_12_ascii_large() {
    assert_eq!(
      parse_numeric(*b"76400000000\x00").ok(),
      Some(8000 * 1024 * 1024),
    )
  }

  #[test]
  fn test_parse_numeric_12_ascii_pre_posix() {
    assert_eq!(parse_numeric(*b"         17 ").ok(), Some(15),)
  }
}
