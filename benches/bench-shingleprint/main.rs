use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tarcrush::shingleprint;
use std::time::Duration;

const INPUT_BINSORT1KB: &'static [u8] = include_bytes!("input-binsort1kB.dat");
const INPUT_LINUX1MB: &'static [u8] = include_bytes!("input-linux1MB.dat");

fn bench_shingleprint(c: &mut Criterion) {
  let mut g = c.benchmark_group("bench_shingleprint");
  g.measurement_time(Duration::from_secs(15));
  g.bench_function(
    "shingleprint_portable_binsort1kB",
    |b| b.iter(|| shingleprint::shingleprint_portable(black_box(INPUT_BINSORT1KB))),
  );
  g.bench_function(
    "shingleprint_portable_linux1MB",
    |b| b.iter(|| shingleprint::shingleprint_portable(black_box(INPUT_LINUX1MB))),
  );
  if is_x86_feature_detected!("sse4.2") {
    g.bench_function(
      "shingleprint_sse_binsort1kB",
      |b| b.iter(|| unsafe { shingleprint::shingleprint_sse(black_box(INPUT_BINSORT1KB)) }),
    );
    g.bench_function(
      "shingleprint_sse_linux1MB",
      |b| b.iter(|| unsafe { shingleprint::shingleprint_sse(black_box(INPUT_LINUX1MB)) }),
    );
  }
  g.finish();
}

criterion_group!(benches, bench_shingleprint);
criterion_main!(benches);
