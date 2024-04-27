use crate::memory::{map_page_auto, PAGE_SIZE, VirtAddr};

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
        let initial_size = 8192;
        let tree = HeapTree {
            tree_ptr: ptr,
            size: initial_size,
        };
        tree.allocate_pages();

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

    fn get_node_val(&self, node: u32) -> u32 {
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        (unsafe { *self.tree_ptr.add(node as usize) } & 0b01111111) as u32
    }

    fn is_node_taken(&self, node: u32) -> bool {
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        (unsafe { *self.tree_ptr.add(node as usize) } & 0b10000000) != 0
    }

    fn set_node_val(&mut self, node: u32, val: u32) {
        debug_assert!(val < 0b10000000);
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        unsafe {
            *self.tree_ptr.add(node as usize) &= 0b10000000;
            *self.tree_ptr.add(node as usize) |= val as u8;
        }
    }

    fn set_node_taken(&mut self, node: u32, taken: bool) {
        debug_assert!(node < 2 * self.get_tree_size());
        debug_assert!(node != 0);
        unsafe {
            if taken {
                *self.tree_ptr.add(node as usize) |= 0b10000000;
            } else {
                *self.tree_ptr.add(node as usize) &= 0b01111111;
            }
        }
    }

    /// Finds a new region of the size: 2^size
    pub fn alloc(&self, size: u32) -> u32 {
        let size = 1 << size;

        0
    }

    /// Frees a region at the position
    /// If no region is reserved there, it panics
    pub fn free(&self, pos: u32) {}
}
