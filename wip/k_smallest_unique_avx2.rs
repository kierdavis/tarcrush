fn k_smallest_unique_avx2(mut values: impl Iterator<Item = u32>) -> ArrayVec<u32, 32> {
  // Convention: in a __m256i, lane 0 is the rightmost / least-significant lane.

  struct WorkingSet {
    data: [__m256i; 4],
    popmask: [__m256i; 4],
    npop: usize,
  }

  impl WorkingSet {
    #[inline]
    fn new() -> WorkingSet {
      let zero = _mm256_setzero_si256();
      WorkingSet { data: [zero; 4], popmask: [zero; 4], npop: 0 }
    }
    #[inline]
    fn contains(&self, elem_bcast: __m256i) -> bool {
      (
        _mm256_testz_si256(_mm256_cmpeq_epi32(self.data[0], elem_bcast), self.popmask[0]) &
        _mm256_testz_si256(_mm256_cmpeq_epi32(self.data[1], elem_bcast), self.popmask[1]) &
        _mm256_testz_si256(_mm256_cmpeq_epi32(self.data[2], elem_bcast), self.popmask[2]) &
        _mm256_testz_si256(_mm256_cmpeq_epi32(self.data[3], elem_bcast), self.popmask[3])
      ) == 0
    }
    #[inline]
    fn insert(&mut self, elem_bcast: __m256i) {
      debug_assert!(self.npop < 32);
      todo!();
    }
    #[inline]
    fn remove(&mut self, elem_bcast: __m256i) {
      debug_assert!(self.npop > 0);
    }
  }

  todo!()
}
