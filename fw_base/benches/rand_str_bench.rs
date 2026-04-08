use criterion::{Criterion, criterion_group, criterion_main};
use fw_base::utils::rand::rand_str;

fn bench_rand_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("RandStr Generation");

    group.bench_function("RandStr_32", |b| b.iter(|| rand_str(32)));

    group.finish();
}

criterion_group!(benches, bench_rand_str);
criterion_main!(benches);
