use core::ops::Add;
use core::ptr::write_bytes;
use crate::{allocate_page, bitset_size_bytes, deallocate_page, BitSetRaw, Mutable, HEAP_ADDR};
use crate::bitset::{get_raw, set_raw};

pub const HEAP_REGION_SIZE: u64 = 1 << 28;
const PAGE_SIZE: usize = 4096;

struct HeapRegion {
    bitset: BitSetRaw,
    // to keep track of which pages should be reserved
    bitset_page_addr: *mut u8,
    heap_page_addr: *mut u8,

    bitset_base_addr: *mut u8,
    base_addr: *mut u8,
    block_size: usize,
}

impl HeapRegion {
    fn free(&mut self, ptr: *mut u8) {
        let idx = (ptr as u64 - self.base_addr as u64) / self.block_size as u64;
        self.bitset.set(idx as usize, false);
    }

    fn alloc(&mut self) -> *mut u8 {
        if let Some(block) = self.bitset.get_zero_element() {
            self.bitset.set(block, true);
            unsafe { self.base_addr.add(block * self.block_size) }
        } else {
            let size = self.bitset.get_size();
            let new_bitset_page_addr = unsafe { self.bitset_base_addr.add(bitset_size_bytes(size + 1).div_ceil(PAGE_SIZE) * PAGE_SIZE) };
            if new_bitset_page_addr != self.bitset_page_addr {
                allocate_page(self.bitset_page_addr, false);
                self.bitset_page_addr = new_bitset_page_addr;
            }

            let new_heap_page_addr = unsafe { self.base_addr.add(((size + 1) * self.block_size).div_ceil(PAGE_SIZE) * PAGE_SIZE) };
            if new_heap_page_addr != self.heap_page_addr {
                allocate_page(self.heap_page_addr, false);
                self.heap_page_addr = new_heap_page_addr;
            }

            self.bitset.add_one();
            self.bitset.set(size, true);

            unsafe { self.base_addr.add(size * self.block_size) }
        }
    }
}

/// A final region, that allocates entire pages worth of memory
/// Each allocation has its own page(s)
/// Effective for allocations bigger or equal to 1 page
/// Bitset has 2 bits per block. First bit indicates if the block is taken,
/// second bit indicates if the block is the first one in the page.
/// Bitset size is fixed to a whole page, so it can hold info for 4096 * 8 / 2 = 16384 pages,
/// which is 64MB of memory.
struct HeapMegaRegion {
    bitset_addr: *mut u8,
    heap_addr: *mut u8,
    curr_idx: usize,
}

const NUM_MEGA_PAGES: usize = 4096 * 8 / 2;

impl HeapMegaRegion {
    fn get_is_taken(&self, idx: usize) -> bool {
        unsafe { get_raw(self.bitset_addr as *mut u64, idx * 2) }
    }

    fn get_is_first(&self, idx: usize) -> bool {
        unsafe { get_raw(self.bitset_addr as *mut u64, idx * 2 + 1) }
    }

    fn set_is_taken(&self, idx: usize, val: bool) {
        unsafe { set_raw(self.bitset_addr as *mut u64, idx * 2, val) }
    }

    fn set_is_first(&self, idx: usize, val: bool) {
        unsafe { set_raw(self.bitset_addr as *mut u64, idx * 2 + 1, val) }
    }

    fn init(&mut self) {
        allocate_page(self.bitset_addr, false);
        // clear the bitset
        unsafe {
            write_bytes(self.bitset_addr, 0, PAGE_SIZE);
        }
    }

    /// size is in pages
    fn alloc(&mut self, size: usize) -> *mut u8 {
        let mut left_idx = self.curr_idx;
        let mut right_idx = left_idx;
        let mut looped = false;
        while right_idx - left_idx < size {
            if right_idx >= NUM_MEGA_PAGES {
                if looped {
                    panic!("Out of mega pages");
                }
                looped = true;
                left_idx = 0;
                right_idx = 0;
            }

            if self.get_is_taken(right_idx) {
                left_idx = right_idx + 1;
            }
            right_idx += 1;
        }

        for i in left_idx..right_idx {
            self.set_is_taken(i, true);
            self.set_is_first(i, i == left_idx);
            let addr = unsafe { self.heap_addr.add(i * PAGE_SIZE) };
            allocate_page(addr, false);
        }

        self.curr_idx = right_idx;
        unsafe { self.heap_addr.add(left_idx * PAGE_SIZE) }
    }

    fn free(&mut self, idx: usize) {
        assert!(self.get_is_taken(idx));
        assert!(self.get_is_first(idx));

        let mut i = idx;
        loop {
            self.set_is_taken(i, false);
            self.set_is_first(i, false);
            let addr = unsafe { self.heap_addr.add(i * PAGE_SIZE) };
            // free the page
            deallocate_page(addr);
            i += 1;

            if i >= NUM_MEGA_PAGES || self.get_is_first(i) || !self.get_is_taken(i) {
                break;
            }
        }
    }
}

const A: HeapRegion = HeapRegion {
    bitset: BitSetRaw::new_empty(),
    bitset_page_addr: 0 as *mut u8,
    heap_page_addr: 0 as *mut u8,
    base_addr: 0 as *mut u8,
    bitset_base_addr: 0 as *mut u8,
    block_size: 0,
};
static HEAP_REGIONS: Mutable<[HeapRegion; 9]> = Mutable::new([A, A, A, A, A, A, A, A, A]);
static HEAP_MEGA_REGION: Mutable<HeapMegaRegion> = Mutable::new(HeapMegaRegion {
    bitset_addr: 0 as *mut u8,
    heap_addr: 0 as *mut u8,
    curr_idx: 0,
});

pub fn init_malloc() {
    let t = HEAP_REGIONS.borrow();

    for i in 0..9 {
        let region = &mut HEAP_REGIONS.get_mut(&t)[i];
        let addr = unsafe { HEAP_ADDR + (2 * i as u64) * HEAP_REGION_SIZE } as *mut u8;
        region.base_addr = unsafe { addr.add(HEAP_REGION_SIZE as usize) };
        region.bitset_base_addr = addr;
        region.bitset = BitSetRaw::new(0, region.bitset_base_addr as *mut u64);
        region.bitset_page_addr = addr;
        region.heap_page_addr = region.base_addr;
        region.block_size = 8 << i;
    }

    HEAP_REGIONS.release(t);

    let t = HEAP_MEGA_REGION.borrow();
    let region = HEAP_MEGA_REGION.get_mut(&t);
    region.bitset_addr = unsafe { (HEAP_ADDR as *mut u8).add(2 * 9 * HEAP_REGION_SIZE as usize) };
    region.heap_addr = unsafe { region.bitset_addr.add(PAGE_SIZE) };
    region.init();
    HEAP_MEGA_REGION.release(t);
}

pub fn malloc(size: usize) -> *mut u8 {
    for i in 0..9 {
        let curr_size = 8 << i;
        if curr_size >= size {
            let t = HEAP_REGIONS.borrow();
            let region = &mut HEAP_REGIONS.get_mut(&t)[i];
            let ptr = region.alloc();
            HEAP_REGIONS.release(t);
            return ptr;
        }
    }

    let t = HEAP_MEGA_REGION.borrow();
    let region = HEAP_MEGA_REGION.get_mut(&t);
    let ptr = region.alloc(size.div_ceil(PAGE_SIZE));
    HEAP_MEGA_REGION.release(t);
    ptr
}

pub fn free(ptr: *mut u8) {
    let region_idx = (ptr as u64 - unsafe { HEAP_ADDR }) / (2 * HEAP_REGION_SIZE);
    if region_idx == 9 {
        let t = HEAP_MEGA_REGION.borrow();
        let region = HEAP_MEGA_REGION.get_mut(&t);
        let idx = (ptr as u64 - region.heap_addr as u64) / PAGE_SIZE as u64;
        region.free(idx as usize);
        HEAP_MEGA_REGION.release(t);
        return;
    }

    let t = HEAP_REGIONS.borrow();
    let region = &mut HEAP_REGIONS.get_mut(&t)[region_idx as usize];
    region.free(ptr);
    HEAP_REGIONS.release(t);
}