use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

fn bench_uuids(c: &mut Criterion) {
    let mut group = c.benchmark_group("UUID Generation");

    group.bench_function("v7_timestamp", |b| {
        b.iter(|| Uuid::now_v7())
    });

    group.finish();
}

criterion_group!(benches, bench_uuids);
criterion_main!(benches);