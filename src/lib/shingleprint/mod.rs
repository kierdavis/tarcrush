use crate::util::k_smallest_unique::k_smallest_unique;
use arrayvec::ArrayVec;

pub use crate::tunables::{SHINGLEPRINT_FEATURES, SHINGLE_LEN};

pub mod hash;

// Invariant: array elements are sorted in ascending order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shingleprint(ArrayVec<hash::ShingleHash, SHINGLEPRINT_FEATURES>);

pub fn shingleprint_portable(input: &[u8]) -> Shingleprint {
  let shingles = input.windows(SHINGLE_LEN);
  let hashes = shingles.map(hash::hash_portable);
  let hashes = k_smallest_unique::<_, SHINGLEPRINT_FEATURES>(hashes);
  Shingleprint(hashes.into_iter().collect())
}

// Undefined behaviour if the processor doesn't support the sse4.2 feature.
#[target_feature(enable = "sse4.2")]
pub unsafe fn shingleprint_sse(input: &[u8]) -> Shingleprint {
  let shingles = input.windows(SHINGLE_LEN);
  let hashes = shingles.map(|s| unsafe { hash::hash_sse(s) });
  let hashes = k_smallest_unique::<_, SHINGLEPRINT_FEATURES>(hashes);
  Shingleprint(hashes.into_iter().collect())
}

pub fn shingleprint(input: &[u8]) -> Shingleprint {
  if is_x86_feature_detected!("sse4.2") {
    unsafe { shingleprint_sse(input) }
  } else {
    shingleprint_portable(input)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  const INPUT1: &'static [u8] =
    b"The quick brown fox jumps over the lazy dog, and jumps over the lazy dog once more.";
  const EXPECTED_OUTPUT1: [u32; 32] = [
    0x033587c5, // "umps over the la"
    0x05b12fee, // "e quick brown fo"
    0x1980c846, // "he quick brown f"
    0x1d48713a, // " jumps over the "
    0x1fa20db9, // ", and jumps over"
    0x2204152a, // "zy dog once more"
    0x29f55ece, // "e lazy dog, and "
    0x2cc7491e, // "g, and jumps ove"
    0x2d1beb0d, // "og, and jumps ov"
    0x33ef503d, // "ps over the lazy"
    0x35c43c8a, // "er the lazy dog,"
    0x375dfa1a, // "brown fox jumps "
    0x37e9db4b, // "ver the lazy dog"
    0x38ce842c, // " quick brown fox"
    0x3bf9991e, // "The quick brown "
    0x455e775e, // "r the lazy dog, "
    0x45faa7d6, // "own fox jumps ov"
    0x4a426dd6, // "and jumps over t"
    0x52ce507b, // "rown fox jumps o"
    0x540bc1a1, // "fox jumps over t"
    0x57c9d9d6, // " fox jumps over "
    0x5a57cc7d, // "quick brown fox "
    0x5e6e5527, // "e lazy dog once "
    0x5eb5b1d8, // "azy dog, and jum"
    0x60b5b863, // "jumps over the l"
    0x660da061, // "ck brown fox jum"
    0x66f5e2c2, // "wn fox jumps ove"
    0x6d6879ac, // "n fox jumps over"
    0x6e48c13b, // "dog, and jumps o"
    0x7358b062, // "azy dog once mor"
    0x73a84068, // " and jumps over "
    0x74c764b5, // " the lazy dog on"
  ];

  #[test]
  fn test_portable() {
    assert_eq!(
      shingleprint_portable(INPUT1),
      Shingleprint(EXPECTED_OUTPUT1.into()),
    );
  }

  #[test]
  fn test_sse() {
    if is_x86_feature_detected!("sse4.2") {
      assert_eq!(
        unsafe { shingleprint_sse(INPUT1) },
        Shingleprint(EXPECTED_OUTPUT1.into()),
      );
    }
  }
}
