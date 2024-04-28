use crate::memory::{map_page_auto, memcpy, memcpy_non_aligned, memset, PAGE_SIZE, VirtAddr};
use crate::println;

/// Heap tree is a data structure, that keeps track of free regions on the heap
/// It does not use malloc/free, because it is used in malloc and free (obviously)
/// You need to pass it a pointer where it lives and it will automatically allocate pages and resize itself as it can
/// This tree has the following capabilities:
/// - find and occupy a region that has the size 2^n and is also 2^n aligned
/// - if there is no region available, it automatically doubles its size
pub struct HeapTree {
    tree_ptr: *mut u8,
    // the length of the array
    size: u32,
}

impl HeapTree {
    pub unsafe fn new(ptr: *mut u8) -> Self {
        let mut tree = HeapTree {
            tree_ptr: ptr,
            size: 8192,
        };
        tree.allocate_pages();
        tree.clear();
        tree
    }

    /// returns n - the size of the tree
    /// the number of nodes in the tree is then 2 * n - 1
    fn get_tree_size(&self) -> u32 {
        self.size / 8
    }

    fn get_base_ptr(&self) -> *mut u8 {
        unsafe { self.tree_ptr.add(self.get_tree_size() as usize * 2) }
    }

    fn allocate_pages(&self) {
        let from = self.get_base_ptr() as u64 / PAGE_SIZE;
        let to = (self.get_base_ptr() as u64 + self.get_tree_size() as u64 * 3 + PAGE_SIZE - 1)
            / PAGE_SIZE;
        for page in from..to {
            map_page_auto((page * PAGE_SIZE) as VirtAddr, true, false);
        }
    }

    fn get_node_val(&self, node: u32) -> i32 {
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        unsafe { *self.tree_ptr.add(node as usize) as i32 - 1 }
    }

    fn set_node_val(&mut self, node: u32, val: i32) {
        debug_assert!(val < (1 << 8) - 1);
        debug_assert!(val >= -1);
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        unsafe {
            *self.tree_ptr.add(node as usize) = (val + 1) as u8;
        }
    }

    /// Makes all regions free
    pub fn clear(&mut self) {
        unsafe {
            memset(self.get_base_ptr(), 0, self.get_tree_size() as usize);
        }
        let mut curr = self.get_tree_size();
        let mut curr_val = 3;
        while curr != 0 {
            unsafe {
                memset(self.tree_ptr.add(curr as usize), curr_val, curr as usize);
            }
            curr_val += 1;
            curr /= 2;
        }
    }

    fn merge(a: i32, b: i32, csize: i32) -> i32 {
        if a == csize - 1 && b == csize - 1 {
            csize
        } else {
            i32::max(a, b)
        }
    }

    fn update_node(&mut self, node: u32, size_log2: u32) {
        self.set_node_val(
            node,
            Self::merge(
                self.get_node_val(2 * node),
                self.get_node_val(2 * node + 1),
                size_log2 as i32,
            ),
        );
    }

    /// Doubles its size
    fn double_size(&mut self) {
        let prev_base_ptr = self.get_base_ptr();
        self.size *= 2;
        println!("Resized to {}", self.size);
        self.allocate_pages();

        unsafe {
            memset(self.get_base_ptr(), 0, self.get_tree_size() as usize);
            memcpy(
                prev_base_ptr,
                self.get_base_ptr(),
                self.get_tree_size() as usize / 2,
            );
        }

        let mut curr = self.get_tree_size();
        let mut curr_val = 3;
        while curr != 1 {
            unsafe {
                memset(self.tree_ptr.add(curr as usize), curr_val, curr as usize);
                memcpy_non_aligned(
                    self.tree_ptr.add(curr as usize / 2),
                    self.tree_ptr.add(curr as usize),
                    curr as usize / 2,
                );
            }
            curr_val += 1;
            curr /= 2;
        }
        self.update_node(1, curr_val as u32);
    }

    fn get_biggest_segment(bits: u32) -> i32 {
        debug_assert!(bits < (1 << 8));

        if bits == 0b11111111 {
            return -1;
        }

        if bits == 0 {
            return 3;
        }

        for size in (0..=2).rev() {
            let size2 = 1 << size;
            let bits2: u32 = (1 << size2) - 1;
            for i in 0..8 / size2 {
                if (bits & (bits2 << (i * size2))) == 0 {
                    return size;
                }
            }
        }
        println!("Bits {:#010b}", bits);
        unreachable!();
    }

    /// Finds a new region of the size: 2^size
    pub fn alloc(&mut self, size_log2: u32) -> u32 {
        debug_assert!(size_log2 < 48);
        let size = 1 << size_log2;

        // increase size, until there is enough space
        while self.get_node_val(1) < size_log2 as i32 {
            self.double_size();
        }

        let mut node = 1;
        let mut l = 0;
        let mut r = self.size;

        loop {
            if r - l == size {
                // this node will be allocated
                self.set_node_val(node, -1);

                // update all parent nodes
                let mut csize_log2 = size_log2 + 1;
                node /= 2;
                while node != 0 {
                    self.update_node(node, csize_log2);
                    node /= 2;
                    csize_log2 += 1;
                }
                return l;
            }

            // we cannot go down any further, which means we will have to deal with a bitmask array of u8
            if node >= self.get_tree_size() {
                break;
            }

            let mid = (l + r) / 2;
            if self.get_node_val(2 * node) >= size_log2 as i32 {
                node = 2 * node;
                r = mid;
            } else {
                node = 2 * node + 1;
                l = mid;
            }
        }

        let idx = node - self.get_tree_size();
        let bitmask = unsafe { *self.get_base_ptr().add(idx as usize) as u32 };
        let bits: u32 = (1 << size) - 1;
        for i in 0..8 / size {
            if (bitmask & (bits << (i * size))) == 0 {
                unsafe {
                    *self.get_base_ptr().add(idx as usize) |= (bits << (i * size)) as u8;
                }

                let node_val = Self::get_biggest_segment(unsafe {
                    *self.get_base_ptr().add(idx as usize) as u32
                });
                self.set_node_val(node, node_val);

                // update all parent nodes
                let mut csize_log2 = 4;
                node /= 2;
                while node != 0 {
                    self.update_node(node, csize_log2);
                    node /= 2;
                    csize_log2 += 1;
                }

                return 8 * idx + i * size;
            }
        }

        println!("Node {}", node);
        println!("Node val {}", self.get_node_val(node));
        println!("Bits {:#010b}", bitmask);

        unreachable!();
    }

    /// Frees a region at the position
    /// If no region is reserved there, it panics
    pub fn free(&mut self, pos: u32) {}
}
