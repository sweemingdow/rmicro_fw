use criterion::{Criterion, criterion_group, criterion_main};
use fw_crypto::aes::AesKeyDisplayType;
use fw_crypto::aes::gcm::{AesGcm, gen_gcm_256_key_as_hex};

fn gcm_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("AEC_GCM Generation");

    let key = gen_gcm_256_key_as_hex();

    let plain = "Hello AES-256-CBC！中文测试 🎉@@fsdf";

    group.bench_function("aes_gcm", move |b| {
        let ag = AesGcm::from_str(&key, AesKeyDisplayType::Hex).unwrap();
        b.iter(move || {
            let (cipher, nonce) = ag.encrypt(plain).unwrap();

            let _ = ag.decrypt(&cipher, &nonce).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, gcm_bench);
criterion_main!(benches);
