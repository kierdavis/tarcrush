use std::collections::BTreeSet;

pub fn k_smallest_unique<T, const K: usize>(values: impl Iterator<Item=T>) -> BTreeSet<T>
where
  T: Ord,
{
  let mut smallest = BTreeSet::new();
  for candidate in values {
    if smallest.len() == K && *smallest.last().unwrap() <= candidate {
      // We've already retained K values that are smaller than or equal to this candidate.
      continue
    }
    if smallest.contains(&candidate) {
      continue
    }
    // This candidate is smaller than at least one member of the working set,
    // and is not a duplicate of any member, so insert it.
    if smallest.len() == K {
      smallest.pop_last(); // Make room.
    }
    smallest.insert(candidate);
  }
  smallest
}
