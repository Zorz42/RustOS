use kernel_test::{kernel_test, kernel_test_mod};

use crate::memory::{HeapTree, TESTING_OFFSET};
use crate::rand::Rng;
use crate::println;

kernel_test_mod!(crate::tests::A4_heap_tree);

const HEAP_TREE_PTR: u64 = TESTING_OFFSET;

fn get_heap_tree() -> HeapTree {
    unsafe { HeapTree::new(HEAP_TREE_PTR as *mut u8) }
}

#[kernel_test]
fn test_heap_tree_init() {
    let _tree = get_heap_tree();
}

#[kernel_test]
fn test_heap_tree_alloc() {
    let mut tree = get_heap_tree();
    let mut rng = Rng::new(5473895743);

    for _ in 0..1000 {
        let _ = tree.alloc(rng.get(0, 8) as u32);
    }
}

#[kernel_test]
fn test_heap_tree_alloc_free() {
    let mut tree = get_heap_tree();
    let mut rng = Rng::new(5473895743);

    for _ in 0..100000 {
        let pos = tree.alloc(rng.get(0, 8) as u32);
        assert!(pos < 10000);
        tree.free(pos);
    }
}

#[kernel_test]
fn test_heap_tree_alloc_free_batch() {
    let mut tree = get_heap_tree();
    let mut rng = Rng::new(6436534);

    for _ in 0..100 {
        let mut arr = [0; 1024];
        for i in 0..1024 {
            arr[i] = tree.alloc(rng.get(0, 8) as u32);
            assert!(arr[i] < 1000000);
        }

        // create a random permutation
        let mut perm = [0; 1024];
        for i in 0..1024 {
            perm[i] = i;
        }
        for i1 in 0..1024 {
            let i2 = rng.get(0, 1024) as usize;
            let temp = perm[i1];
            perm[i1] = perm[i2];
            perm[i2] = temp;
        }

        for i in 0..1024 {
            tree.free(arr[perm[i]]);
        }
    }
}

#[kernel_test]
fn test_heap_tree_alloc_aligned() {
    let mut tree = get_heap_tree();
    let mut rng = Rng::new(5473895743);

    for _ in 0..10000 {
        let size = rng.get(0, 8) as u32;
        let pos = tree.alloc(size);

        assert_eq!(pos % (1 << size), 0);

        tree.free(pos);
    }
}

#[kernel_test]
fn test_heap_tree_alloc_disjoint() {
    let mut tree = get_heap_tree();
    let mut rng = Rng::new(5473895743);

    let mut arr = [(0, 0); 1000];

    for i in 0..1000 {
        arr[i].1 = rng.get(0, 8) as u32;
        arr[i].0 = tree.alloc(arr[i].1);
    }

    for i1 in 0..1000 {
        for i2 in 0..1000 {
            if i1 == i2 {
                continue;
            }
            let l1 = arr[i1].0;
            let r1 = l1 + (1 << arr[i1].1);
            let l2 = arr[i2].0;
            let r2 = l2 + (1 << arr[i2].1);
            assert!(r2 <= l1 || r1 <= l2);
        }
    }

    for i in 0..1000 {
        tree.free(arr[i].0);
    }
}
