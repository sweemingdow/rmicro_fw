use criterion::{Criterion, criterion_group, criterion_main};
use fw_crypto::aes::{AesBitsType, AesKeyDisplayType};
use fw_crypto::aes::cbc::{AesCbc, gen_cbc_256_key_as_hex, gen_iv_as_hex};

fn cbc_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("AEC_CBC Generation");
    let key = gen_cbc_256_key_as_hex();
    let iv = gen_iv_as_hex();

    let plaintext = "Hello AES-256-CBC！中文测试 🎉";

    group.bench_function("aes_cbc", move |b| {
        let ac = AesCbc::new(&key, &iv, AesBitsType::Bits256, AesKeyDisplayType::Hex).unwrap();
        b.iter(move || {
            let ciphertext = ac.encrypt(plaintext).unwrap();

            let _ = ac.decrypt(&ciphertext).unwrap();
        })
    });

    group.finish();
}

criterion_group!(benches, cbc_bench);
criterion_main!(benches);
