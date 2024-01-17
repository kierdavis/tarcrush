use std::collections::BTreeSet;

// Without the inline(always) attribute, Rust generates a single
// monomorphisation of this function and then discovers it can't inline it
// into both the portable and SSE-enabled contexts, leading to one of those
// shingleprint implementations being under-optimised.
#[inline(always)]
pub fn k_smallest_unique<T, const K: usize>(mut values: impl Iterator<Item = T>) -> BTreeSet<T>
where
  T: Ord,
{
  let mut working_set = BTreeSet::new();
  for candidate in &mut values {
    debug_assert!(working_set.len() < K);
    working_set.insert(candidate);
    if working_set.len() == K {
      break;
    }
  }
  for candidate in values {
    debug_assert_eq!(working_set.len(), K);
    if *working_set.last().unwrap() <= candidate {
      // We've already retained K values that are smaller than or equal to this candidate.
      continue;
    }
    if working_set.contains(&candidate) {
      continue;
    }
    // This candidate is smaller than at least one member of the working set,
    // and is not a duplicate of any member, so insert it.
    working_set.pop_last(); // Make room.
    working_set.insert(candidate);
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
    let got: Vec<u8> = k_smallest_unique::<_, 12>(input).into_iter().collect();
    const EXPECTED: &'static [u8] = b" .Tabcdefghi";
    assert_eq!(got.as_slice(), EXPECTED);
  }
}
