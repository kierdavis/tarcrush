use criterion::{BenchmarkId, Throughput, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use tarcrush::shingleprint;

const INPUTS: &'static [(&'static str, &'static [u8])] = &[
  ("binsort1kB", include_bytes!("input-binsort1kB.dat")),
  ("linux1MB", include_bytes!("input-linux1MB.dat")),
];

fn bench_shingleprint(c: &mut Criterion) {
  let mut g = c.benchmark_group("shingleprint");
  g.measurement_time(Duration::from_secs(20));
  for &(input_name, input) in INPUTS {
    g.throughput(Throughput::Bytes(input.len() as u64));
    g.bench_with_input(BenchmarkId::new("portable", input_name), input, |b, input| {
      b.iter(|| shingleprint::shingleprint_portable(input))
    });
    if is_x86_feature_detected!("sse4.2") {
      g.bench_with_input(BenchmarkId::new("sse", input_name), input, |b, input| {
        b.iter(|| unsafe { shingleprint::shingleprint_sse(input) })
      });
    }
  }
  g.finish();
}

criterion_group!(benches, bench_shingleprint);
criterion_main!(benches);
