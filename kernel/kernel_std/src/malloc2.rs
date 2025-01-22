use core::ops::Add;
use crate::{allocate_page, bitset_size_bytes, BitSetRaw, Mutable, HEAP_ADDR2};

pub const HEAP_REGION_SIZE: u64 = 1 << 28;
const PAGE_SIZE: u64 = 4096;

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
    fn free(&mut self) {
        todo!()
    }

    fn alloc(&mut self) -> *mut u8 {
        if let Some(block) = self.bitset.get_zero_element() {
            self.bitset.set(block, true);
            unsafe { self.base_addr.add(block * self.block_size) }
        } else {
            let size = self.bitset.get_size() + 1;
            let new_bitset_page_addr = unsafe { self.bitset_base_addr.add(bitset_size_bytes(size + 1).div_ceil(PAGE_SIZE as usize)) };
            if new_bitset_page_addr != self.bitset_page_addr {
                allocate_page(self.bitset_page_addr, false);
                self.bitset_page_addr = new_bitset_page_addr;
            }

            let new_heap_page_addr = unsafe { self.base_addr.add((size * self.block_size).div_ceil(PAGE_SIZE as usize)) };
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

const A: HeapRegion = HeapRegion {
    bitset: BitSetRaw::new_empty(),
    bitset_page_addr: 0 as *mut u8,
    heap_page_addr: 0 as *mut u8,
    base_addr: 0 as *mut u8,
    bitset_base_addr: 0 as *mut u8,
    block_size: 0,
};
static HEAP_REGIONS: Mutable<[HeapRegion; 9]> = Mutable::new([A, A, A, A, A, A, A, A, A]);

pub fn init_malloc2() {
    let t = HEAP_REGIONS.borrow();

    for i in 0..9 {
        let region = &mut HEAP_REGIONS.get_mut(&t)[i];
        let addr = unsafe { HEAP_ADDR2 + (2 * i as u64) * HEAP_REGION_SIZE } as *mut u8;
        region.base_addr = unsafe { addr.add(HEAP_REGION_SIZE as usize) };
        region.bitset_base_addr = addr;
        region.bitset = BitSetRaw::new(0, region.bitset_base_addr as *mut u64);
        region.bitset_page_addr = addr;
        region.heap_page_addr = region.base_addr;
        region.block_size = 8 << i;
    }

    HEAP_REGIONS.release(t);
}

pub fn malloc2(size: usize) -> *mut u8 {
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

    todo!()
}

pub unsafe fn free2(ptr: *mut u8) {
    // TODO: implement
}