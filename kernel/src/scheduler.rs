use core::arch::asm;
use core::ptr::{copy, write_bytes};
use kernel_std::{debug, debugln, println, Lock, Mutable, String, Vec};
use crate::boot::NUM_CORES;
use crate::disk::filesystem::read_file;
use crate::elf::{verify_elf_header, ElfHeader, ElfProgramHeader};
use crate::memory::{create_page_table, clear_page_table, map_page_auto, switch_to_page_table, PageTable, VirtAddr, KERNEL_VIRTUAL_TOP, PAGE_SIZE, USER_CONTEXT, USER_STACK, USER_STACK_SIZE, refresh_paging, virt_to_phys};
use crate::print::check_screen_refresh_for_print;
use crate::riscv::{get_core_id, get_sstatus, interrupts_enable, set_sstatus, SSTATUS_SPP, SSTATUS_UIE};
use crate::timer::get_ticks;
use crate::trap::switch_to_user_trap;

#[derive(Debug)]
#[repr(C)]
pub struct Context {
    pub ra: u64,
    pub sp: u64,
    pub gp: u64,
    pub tp: u64,
    pub t0: u64,
    pub t1: u64,
    pub t2: u64,
    pub s0: u64,
    pub s1: u64,
    pub a0: u64,
    pub a1: u64,
    pub a2: u64,
    pub a3: u64,
    pub a4: u64,
    pub a5: u64,
    pub a6: u64,
    pub a7: u64,
    pub s2: u64,
    pub s3: u64,
    pub s4: u64,
    pub s5: u64,
    pub s6: u64,
    pub s7: u64,
    pub s8: u64,
    pub s9: u64,
    pub s10: u64,
    pub s11: u64,
    pub t3: u64,
    pub t4: u64,
    pub t5: u64,
    pub t6: u64,
    pub pc: u64,
    pub kernel_hartid: u64,
}

#[derive(Clone, Copy)]
pub struct CpuData {
    pub was_last_interrupt_external: bool,
    pub curr_pid: usize,
    pub last_pid: usize,
}

static mut CPU_DATA: [CpuData; NUM_CORES] = [CpuData { was_last_interrupt_external: false, curr_pid: 1000, last_pid: 1000 }; NUM_CORES];

pub fn get_cpu_data() -> &'static mut CpuData {
    unsafe {
        &mut CPU_DATA[get_core_id() as usize]
    }
}

pub fn get_context() -> &'static mut Context {
    unsafe {
        &mut *(USER_CONTEXT as *mut Context)
    }
}

#[derive(PartialEq)]
enum ProcessState {
    Loading, // Its initial phase, loading the program into memory
    Ready, // Ready to be run by a core
    Running, // Is already running on a core
    Sleeping(u64), // Ignore until get_ticks() is greater or equal than the value
}

pub struct Process {
    state: ProcessState,
    needs_paging_refresh: [bool; NUM_CORES],
}

const NUM_PROC: usize = 16;
static mut PROCTABLE: [(Option<Process>, PageTable); NUM_PROC] = [const { (None, 0 as PageTable) }; NUM_PROC];
static PROCTABLE_ALLOC_LOCK: Lock = Lock::new();
static PROCTABLE_LOCKS: [Lock; NUM_PROC] = [const { Lock::new() }; NUM_PROC];

pub fn init_scheduler() {
    unsafe {
        for i in 0..NUM_PROC {
            PROCTABLE[i].1 = create_page_table();
        }
    }
}

fn get_free_proc() -> usize {
    unsafe {
        for i in 0..NUM_PROC {
            if PROCTABLE[i].0.is_none() {
                return i;
            }
        }
    }

    panic!("No free process slots");
}

pub fn run_program(path: &String) {
    let program = read_file(path).unwrap();

    let elf_header = unsafe { (program.as_ptr() as *const ElfHeader).read() };

    if !verify_elf_header(&elf_header) {
        println!("Invalid ELF header");
        return;
    }

    PROCTABLE_ALLOC_LOCK.spinlock();
    let free_proc = get_free_proc();

    PROCTABLE_LOCKS[free_proc].spinlock();
    unsafe {
        PROCTABLE[free_proc].0 = (Some(Process {
            state: ProcessState::Loading,
            needs_paging_refresh: [true; NUM_CORES],
        }));
    }
    let page_table = unsafe { PROCTABLE[free_proc].1 };
    PROCTABLE_LOCKS[free_proc].unlock();
    PROCTABLE_ALLOC_LOCK.unlock();

    switch_to_page_table(page_table);

    // get program headers
    let mut program_headers = Vec::new();
    for i in 0..elf_header.ph_entry_count {
        let program_header = unsafe { (program.as_ptr().add(elf_header.ph_offset as usize) as *const ElfProgramHeader).add(i as usize).read() };
        program_headers.push(program_header);
    }

    // map program headers to memory
    for header in &program_headers {
        if header.p_type == 1 && header.memory_size != 0 {
            #[cfg(feature = "assertions")]
            assert!(header.vaddr >= KERNEL_VIRTUAL_TOP);

            let low_page = header.vaddr / PAGE_SIZE;
            let high_page = (header.vaddr + header.memory_size).div_ceil(PAGE_SIZE);
            for page in low_page..high_page {
                map_page_auto((page * PAGE_SIZE) as VirtAddr, true, true, true, true);
            }

            #[cfg(feature = "assertions")]
            assert!(header.memory_size >= header.file_size);
            let ptr_low = header.vaddr as *mut u8;
            let ptr_mid = (header.vaddr + header.file_size) as *mut u8;
            unsafe {
                copy(program.as_ptr().add(header.offset as usize), ptr_low, header.file_size as usize);
                write_bytes(ptr_mid, 0, (header.memory_size - header.file_size) as usize);
            }
        }
    }

    #[cfg(feature = "assertions")]
    assert_eq!(USER_STACK_SIZE % PAGE_SIZE, 0);
    let stack_pages = USER_STACK_SIZE / PAGE_SIZE;
    let stack_top = USER_STACK + USER_STACK_SIZE;
    for i in 0..stack_pages {
        map_page_auto((USER_STACK + i * PAGE_SIZE) as VirtAddr, true, true, true, false);
    }

    map_page_auto(USER_CONTEXT as VirtAddr, true, true, false, false);
    unsafe {
        write_bytes(USER_CONTEXT as *mut u8, 0, size_of::<Context>());
    }

    get_context().pc = elf_header.entry;
    get_context().sp = stack_top;

    let t = NUM_PROCESSES.borrow();
    *NUM_PROCESSES.get_mut(&t) += 1;
    NUM_PROCESSES.release(t);

    PROCTABLE_LOCKS[free_proc].spinlock();
    unsafe {
        PROCTABLE[free_proc].0.as_mut().unwrap().state = ProcessState::Ready;
    }
    PROCTABLE_LOCKS[free_proc].unlock();
}

extern "C" {
    fn jump_to_user() -> !;
}

pub fn scheduler_next_proc() {
    get_cpu_data().curr_pid += 1;
    get_cpu_data().curr_pid %= NUM_PROC;
}

static mut SCHEDULER_ENABLED: bool = true;

pub fn toggle_scheduler(enabled: bool) {
    unsafe {
        SCHEDULER_ENABLED = enabled;
    }
}

static NUM_PROCESSES: Mutable<usize> = Mutable::new(0);

pub fn get_num_processes() -> usize {
    let t = NUM_PROCESSES.borrow();
    let res = *NUM_PROCESSES.get(&t);
    NUM_PROCESSES.release(t);
    res
}

pub fn scheduler() -> ! {
    let mut misses = 0;
    loop {
        unsafe {
            if !SCHEDULER_ENABLED {
                asm!("wfi");
                continue;
            }
        }

        if misses == NUM_PROC {
            misses = 0;
            check_screen_refresh_for_print();
            unsafe {
                asm!("wfi");
            }
        }

        let pid = get_cpu_data().curr_pid;

        PROCTABLE_LOCKS[pid].spinlock();

        unsafe {
            if let Some(ProcessState::Sleeping(until)) = PROCTABLE[pid].0.as_ref().map(|p| &p.state) {
                if *until <= get_ticks() {
                    PROCTABLE[pid].0.as_mut().unwrap().state = ProcessState::Ready;
                }
            }

            if PROCTABLE[pid].0.is_none() || PROCTABLE[pid].0.as_ref().unwrap().state != ProcessState::Ready {
                misses += 1;
                scheduler_next_proc();
                PROCTABLE_LOCKS[pid].unlock();
                continue;
            }
        }

        interrupts_enable(false);

        // clear bit in sstatus
        set_sstatus(get_sstatus() & !SSTATUS_SPP);

        // set user interrupt enable
        set_sstatus(get_sstatus() | SSTATUS_UIE);

        switch_to_user_trap();

        unsafe {
            switch_to_page_table(PROCTABLE[pid].1);

            if PROCTABLE[pid].0.as_ref().unwrap().needs_paging_refresh[get_core_id() as usize] {
                refresh_paging();
                PROCTABLE[pid].0.as_mut().unwrap().needs_paging_refresh[get_core_id() as usize] = false;
            }

            PROCTABLE[pid].0.as_mut().unwrap().state = ProcessState::Running;
            get_cpu_data().last_pid = pid;
            PROCTABLE_LOCKS[pid].unlock();

            jump_to_user();
        }
    }
}

pub fn mark_process_ready(pid: usize) {
    PROCTABLE_LOCKS[pid].spinlock();

    unsafe {
        PROCTABLE[pid].0.as_mut().unwrap().state = ProcessState::Ready;
    }

    PROCTABLE_LOCKS[pid].unlock();
}

pub fn put_process_to_sleep(pid: usize, until: u64) {
    PROCTABLE_LOCKS[pid].spinlock();

    unsafe {
        PROCTABLE[pid].0.as_mut().unwrap().state = ProcessState::Sleeping(until);
    }

    PROCTABLE_LOCKS[pid].unlock();
}

pub fn terminate_process(pid: usize) {
    PROCTABLE_LOCKS[pid].spinlock();

    unsafe {
        clear_page_table(PROCTABLE[pid].1);
        refresh_paging();
        PROCTABLE[pid].0 = None;
    }

    let t = NUM_PROCESSES.borrow();
    *NUM_PROCESSES.get_mut(&t) -= 1;
    NUM_PROCESSES.release(t);

    PROCTABLE_LOCKS[pid].unlock();
}

pub fn refresh_paging_for_proc(pid: usize) {
    PROCTABLE_LOCKS[pid].spinlock();

    unsafe {
        PROCTABLE[pid].0.as_mut().unwrap().needs_paging_refresh = [true; NUM_CORES];
    }

    PROCTABLE_LOCKS[pid].unlock();
}
