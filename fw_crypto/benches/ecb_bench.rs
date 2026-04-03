use criterion::{Criterion, criterion_group, criterion_main};
use fw_crypto::aes::{AesBitsType, AesKeyDisplayType};
use fw_crypto::aes::ecb::{AesEcb, gen_ecb_256_key_with_b64};

fn ecb_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("AEC_ECB Generation");
    let key = gen_ecb_256_key_with_b64();
    let plaintext = "Hello AES-256-CBC！中文测试 🎉@@fsdf";

    group.bench_function("aes_ecb", move |b| {
        let ae = AesEcb::new(&key, AesBitsType::Bits256, AesKeyDisplayType::B64).unwrap();
        b.iter(move || {
            let ciphertext = ae.encrypt(plaintext).unwrap();

            let _ = ae.decrypt(&ciphertext).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, ecb_bench);
criterion_main!(benches);
