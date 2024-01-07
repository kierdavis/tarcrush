// CRC-32 implementation.

pub type ShingleHash = u32;

// The least significant bit of the first byte of the message is considered to
// be the coefficient of the highest-power term in the message polynomial.

// The least significant bit of DIVISOR is the coefficient of x^31 in the
// divisor polynomial, and the most significant bit is the coefficient of x^0.
// The coefficient of x^32 is implied to be 1.
// This divisor is chosen to be the same as that used by x86_64 SSE4.2
// hardware acceleration.
const DIVISOR: u32 = 0x82F63B78;

// N is the number of entries in the lookup table, which must be a power of 2.
// The returned lookup table can be used to hash log2(N) bits of the message in one go.
const fn generate_lut<const N: usize>() -> [u32; N] {
  let mut lut = [0u32; N];
  let mut msg_frag = 0;
  while msg_frag < N {
    let mut accum = msg_frag as u32;
    let mut bit = 1;
    while bit < N {
      let bit_shifted_out = accum & 1 != 0;
      accum >>= 1;
      if bit_shifted_out { accum ^= DIVISOR; }
      bit <<= 1;
    }
    lut[msg_frag] = accum;
    msg_frag += 1;
  }
  lut
}
const LUT8: [u32; 256] = generate_lut::<256>();
const LUT16: [u32; 65536] = generate_lut::<65536>();

pub fn hash_portable(input: &[u8]) -> ShingleHash {
  let mut accum = u32::MAX;
  let mut chunks = input.chunks_exact(2); // of 16 bits
  for chunk in chunks.by_ref() {
    let chunk = u16::from_le_bytes(chunk.try_into().unwrap());
    accum = LUT16[((accum as u16) ^ chunk) as usize] ^ (accum >> 16);
  }
  for &byte in chunks.remainder() {
    accum = LUT8[((accum as u8) ^ byte) as usize] ^ (accum >> 8);
  }
  !accum
}

// Undefined behaviour if the processor doesn't support the sse4.2 feature.
#[target_feature(enable = "sse4.2")]
pub unsafe fn hash_sse(input: &[u8]) -> ShingleHash {
  let mut accum = u64::MAX; // upper 32 bits are ignored throughout.
  let mut chunks = input.chunks_exact(8); // of 64 bits
  for chunk in chunks.by_ref() {
    let chunk = u64::from_le_bytes(chunk.try_into().unwrap());
    accum = core::arch::x86_64::_mm_crc32_u64(accum, chunk);
  }
  let mut accum = accum as u32;
  for &byte in chunks.remainder() {
    accum = core::arch::x86_64::_mm_crc32_u8(accum, byte);
  }
  !accum
}

#[cfg(test)]
mod tests {
  use super::*;

  const INPUT1: &'static [u8] = b"MOUNTAINAARDVARK";
  const EXPECTED_OUTPUT1: ShingleHash = 0xEC8D5402;

  const INPUT2: &'static [u8] = b"Absentmindedness";
  const EXPECTED_OUTPUT2: ShingleHash = 0x25066ADF;

  #[test]
  fn test_portable() {
    assert_eq!(hash_portable(INPUT1), EXPECTED_OUTPUT1);
    assert_eq!(hash_portable(INPUT2), EXPECTED_OUTPUT2);
  }
  
  #[test]
  fn test_sse() {
    if is_x86_feature_detected!("sse4.2") {
      assert_eq!(unsafe { hash_sse(INPUT1) }, EXPECTED_OUTPUT1);
      assert_eq!(unsafe { hash_sse(INPUT2) }, EXPECTED_OUTPUT2);
    }
  }
}
