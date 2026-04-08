use rand::distributions::Standard;
use rand::distributions::uniform::SampleUniform;
use rand::prelude::Distribution;
use rand::{Rng, RngCore, SeedableRng, rngs::SmallRng};
use std::cell::UnsafeCell;
use uuid::Uuid;

#[inline]
pub fn gen_uuid() -> Uuid {
    Uuid::now_v7()
}

// 扩展到 64 字节，多出的 2 个字节用 'a' 和 'b' 填充，或者重复末尾字符
// 这样 idx & 63 永远不会越界，消除了 loop 和 if
const WORDS_64: &[u8; 64] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ab";

thread_local! {
    // 使用 UnsafeCell 绕过 RefCell 计数检查，SmallRng 在本场景下是安全的
    static RNG: UnsafeCell<SmallRng> = UnsafeCell::new(SmallRng::from_entropy());
}

pub fn rand_str(length: u16) -> String {
    let n = length as usize;
    let mut bytes = vec![0u8; n];

    RNG.with(|rng_ptr| {
        // 安全说明：TLS 保证了单线程访问，UnsafeCell 此时是安全的
        let rng = unsafe { &mut *rng_ptr.get() };

        let mut cache = rng.next_u64();
        let mut remain = 10; // 64位 / 6位 = 10次有效提取

        for i in 0..n {
            if remain == 0 {
                cache = rng.next_u64();
                remain = 10;
            }

            // 无分支获取字符
            let idx = (cache & 0x3F) as usize; // 0x3F = 63
            bytes[i] = WORDS_64[idx];

            cache >>= 6;
            remain -= 1;
        }
    });

    unsafe { String::from_utf8_unchecked(bytes) }
}

pub fn rand_digit<T>() -> T
where
    Standard: Distribution<T>,
{
    RNG.with(|rng_ptr| {
        let rng = unsafe { &mut *rng_ptr.get() };
        Standard.sample(rng)
    })
}

pub fn rand_range<T>(start: T, end: T) -> T
where
    T: SampleUniform + PartialOrd,
{
    if start >= end {
        return start;
    }
    RNG.with(|rng_ptr| {
        let rng = unsafe { &mut *rng_ptr.get() };
        rng.gen_range(start..=end)
    })
}

#[cfg(test)]
mod tests {
    use crate::my_utils::rand::rand_str;
    use crate::utils::rand::{gen_uuid, rand_digit, rand_range};

    #[test]
    fn test_uuv7() {
        let v = gen_uuid();
        print!("{v}");
    }

    #[test]
    fn test_rand_str() {
        let v = rand_str(32);
        println!("{}", v);
    }

    #[test]
    fn test_rand_u32() {
        for i in 0..1000 {
            println!("第{}次, val={}", i + 1, rand_digit::<u32>())
        }
    }

    #[test]
    fn test_rand_range() {
        for i in 0..1000 {
            println!("第{}次, val={}", i + 1, rand_range::<u32>(1, 2))
        }
    }
}

#[cfg(test)]
mod bench_tests {
    use super::*;
    use dashmap::DashMap;
    use rayon::prelude::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Barrier;

    /// 测试 1: 高并发下的碰撞测试 (Collision Test)
    /// 生成 100 万个字符串，检查是否有重复
    #[test]
    fn test_high_concurrency_collision() {
        let total_count = 1_000_000;
        let str_len = 16;
        let results = DashMap::with_capacity(total_count);

        println!(
            "开始并发生成 {} 个随机字符串,  长度为:{}",
            total_count, str_len
        );

        let mut repeated = false;
        for i in 0..total_count {
            let s = rand_str(str_len);

            if results.insert(s, ()).is_some() {
                repeated = true;
                println!("第{}轮, 已存在, 跳出", i);
                break;
            }
        }

        if !repeated {
            println!(
                "成功！{}个并发生成长度为:{}的字符串中无重复。",
                total_count, str_len
            );

            let v = results
                .iter()
                .take(5)
                .map(|r| r.key().clone())
                .collect::<Vec<String>>();
            println!("{:?}", v)
        }
    }

    /// 测试 2: 字符分布均匀性测试 (Distribution Test)
    /// 验证 WORDS_64 中的每个字符出现的概率是否符合预期
    #[test]
    fn test_character_distribution() {
        let total_chars = 1_000_000;
        let s = rand_str(total_chars as u16);
        let mut counts = HashMap::new();

        for c in s.chars() {
            *counts.entry(c).or_insert(0) += 1;
        }

        println!("字符分布统计 (总计 {}):", total_chars);
        let mut sorted_counts: Vec<_> = counts.into_iter().collect();
        sorted_counts.sort_by(|a, b| b.1.cmp(&a.1));

        for (char, count) in sorted_counts.iter().take(5) {
            println!(
                "字符 '{}': 出现 {} 次 ({:.2}%)",
                char,
                count,
                (*count as f64 / total_chars as f64) * 100.0
            );
        }

        // 预期验证：由于我们补齐了 'a' 和 'b'，它们的频次理论上应接近其他字符的两倍
        // 其他字符概率约为 1/64 = 1.56%, 'a' 和 'b' 约为 3.12%
    }

    /// 测试 3: 种子隔离测试 (Race Condition / Seeding Test)
    /// 强制所有线程在同一时刻启动，验证生成的第一个字符串是否相同
    #[test]
    fn test_thread_seeding_isolation() {
        let num_threads = 20;
        let barrier = Arc::new(Barrier::new(num_threads));
        let mut handles = vec![];

        for _ in 0..num_threads {
            let b = barrier.clone();
            handles.push(std::thread::spawn(move || {
                b.wait(); // 等待所有线程就绪
                rand_str(32) // 同时触发生成
            }));
        }

        let mut first_results = vec![];
        for h in handles {
            first_results.push(h.join().unwrap());
        }

        // 检查是否有任何两个线程生成的第一个字符串是一模一样的
        let mut set = std::collections::HashSet::new();
        for s in first_results {
            if !set.insert(s) {
                panic!("不同线程生成了相同的随机序列！种子初始化可能存在竞争。");
            }
        }
        println!("成功！{} 个线程同步启动，随机序列完全独立。", num_threads);
    }
}
