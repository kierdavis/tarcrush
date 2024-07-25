use arrayvec::ArrayVec;

// Without the inline(always) attribute, Rust generates a single
// monomorphisation of this function and then discovers it can't inline it
// into both the portable and SSE-enabled contexts, leading to one of those
// shingleprint implementations being under-optimised.
#[inline(always)]
pub fn k_smallest_unique<T, const K: usize>(mut values: impl Iterator<Item = T>) -> ArrayVec<T, K>
where
  T: Ord,
{
  // Invariant: elements are sorted in ascending order.
  let mut working_set = ArrayVec::new();
  for candidate in &mut values {
    debug_assert!(working_set.len() < K);
    if let Err(insert_idx) = working_set.binary_search(&candidate) {
      working_set.insert(insert_idx, candidate);
      if working_set.len() == K {
        break
      }
    }
  }
  for candidate in values {
    debug_assert_eq!(working_set.len(), K);
    if candidate < *working_set.last().unwrap() {
      if let Err(insert_idx) = working_set.binary_search(&candidate) {
        working_set.truncate(K-1);
        working_set.insert(insert_idx, candidate);
      }
    }
  }
  working_set
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test() {
    let input = b"The quick brown fox jumps over the lazy dog."
      .into_iter()
      .copied();
    let got = k_smallest_unique::<_, 12>(input);
    const EXPECTED: &'static [u8] = b" .Tabcdefghi";
    assert_eq!(got.as_slice(), EXPECTED);
  }
}
