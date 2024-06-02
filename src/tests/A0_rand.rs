use kernel_test::{kernel_test, kernel_test_mod};

use std::Rng;
use crate::println;

kernel_test_mod!(crate::tests::A0_rand);

#[kernel_test]
fn test_rand_in_range() {
    let seeds = [0, 1, 2, 3, 54378395, 4537589435, 25324, 23421];
    for seed in seeds {
        let mut rng = Rng::new(seed);
        let ranges = [
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 4),
            (0, 7),
            (0, 100),
            (0, 1000000000),
            (1, 2),
            (33, 34),
            (54353453, 1345235235),
            (54375353, 54375354),
            (0, (1u64 << 63) - 1 + (1u64 << 63)),
            (1u64 << 63, (1u64 << 63) - 1 + (1u64 << 63)),
            ((1u64 << 63) - 2 + (1u64 << 63), (1u64 << 63) - 1 + (1u64 << 63)),
        ];
        for (l, r) in ranges {
            for _ in 0..100 {
                let val = rng.get(l, r);
                assert!(l <= val);
                assert!(val < r);
            }
        }
    }
}

// this test is probabilistic, so it might fail
#[kernel_test]
fn test_rand_distribution() {
    let seeds = [0, 1, 2, 3, 54378395, 4537589435, 25324, 23421];
    for seed in seeds {
        let mut rng = Rng::new(seed);
        let mods = [2, 4, 5, 6, 7, 8, 17, 32, 124];
        let count = 1000;
        for m in mods {
            let mut counts = [0; 1024];
            for _ in 0..count * m {
                counts[rng.get(0, m) as usize] += 1;
            }
            let min_count = {
                let mut min_count = count; // Dirichlet's principle
                for i in 0..m {
                    min_count = min_count.min(counts[i as usize]);
                }
                min_count
            };
            let max_count = {
                let mut max_count = count; // also Dirichlet's principle
                for i in 0..m {
                    max_count = max_count.max(counts[i as usize]);
                }
                max_count
            };
            assert!(max_count - min_count < 50 * m);
            assert!(min_count > 50);
        }
    }
}
