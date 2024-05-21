use core::arch::asm;
use core::mem::size_of;

pub fn swap<T>(val1: &mut T, val2: &mut T) {
    let ptr1 = val1 as *mut T;
    let ptr2 = val2 as *mut T;

    unsafe {
        asm!("
        mov r11, r8
        add r11, r10
        2:
        mov r10b, [r8]
        xor [r9], r10b
        xor r10b, [r9]
        xor [r9], r10b
        mov [r8], r10b
        add r8, 1
        add r9, 1
        cmp r8, r11
        jne 2b
        
        ", in("r8") ptr1, in("r9") ptr2, in("r10") size_of::<T>(), lateout("r8") _, lateout("r9") _, lateout("r10") _, out("r11") _, options(preserves_flags, nostack));
    }
}

pub unsafe fn volatile_store_byte(ptr: *mut u8, value: u8) {
    asm!("mov [{}], {}", in(reg) ptr, in(reg_byte) value);
}

pub unsafe fn volatile_load_byte(ptr: *mut u8) -> u8 {
    let mut res = 0;
    asm!("mov [{}], {}", in(reg) ptr, out(reg_byte) res);
    res
}

pub unsafe fn memcpy(src: *const u8, dst: *mut u8, len: usize) {
    // check for alignment
    debug_assert_eq!(src as u64 % 8, 0);
    debug_assert_eq!(dst as u64 % 8, 0);
    debug_assert_eq!(len % 8, 0);

    if len == 0 {
        return;
    }

    // check for non-overlapping
    debug_assert!(src as u64 + len as u64 <= dst as u64 || src as u64 >= dst as u64 + len as u64);

    asm!("
    mov r11, r8
    add r11, r10
    2:
    mov r10, [r8]
    mov [r9], r10
    add r8, 8
    add r9, 8
    cmp r8, r11
    jne 2b
    
    ", in("r8") src, in("r9") dst, in("r10") len, lateout("r8") _, lateout("r9") _, lateout("r10") _, out("r11") _, options(preserves_flags, nostack));
}

pub unsafe fn memcpy_non_aligned(src: *const u8, dst: *mut u8, len: usize) {
    // check for non-overlapping
    debug_assert!(src as u64 + len as u64 <= dst as u64 || src as u64 >= dst as u64 + len as u64);

    if len == 0 {
        return;
    }

    asm!("
    mov r11, r8
    add r11, r10
    2:
    mov r10b, [r8]
    mov [r9], r10b
    add r8, 1
    add r9, 1
    cmp r8, r11
    jne 2b
    
    ", in("r8") src, in("r9") dst, in("r10") len, lateout("r8") _, lateout("r9") _, lateout("r10") _, out("r11") _, options(preserves_flags, nostack));
}

pub unsafe fn memset_int64(ptr: *mut u8, val: u64, len: usize) {
    debug_assert_eq!(len % 8, 0);
    debug_assert_eq!(len as u64 % 8, 0);
    if len == 0 {
        return;
    }

    asm!("
    add r9, r8
    2:
    mov [r8], r10
    add r8, 8
    cmp r8, r9
    jne 2b
    
    ", in("r8") ptr, in("r9") len, in("r10") val, lateout("r8") _, lateout("r9") _, lateout("r10") _, options(preserves_flags, nostack));
}

pub unsafe fn memset(mut ptr: *mut u8, val: u8, len: usize) {
    let mut len = len;
    while len > 0 && ptr as u64 % 8 != 0 {
        *ptr = val;
        ptr = (ptr as u64 + 1) as *mut u8;
        len -= 1;
    }
    let mut val_u64 = 0;
    for i in 0..8 {
        val_u64 += (val as u64) << (i * 8);
    }
    let rlen = len / 8 * 8;
    memset_int64(ptr, val_u64, rlen);
    ptr = (ptr as u64 + rlen as u64) as *mut u8;
    let mut len = len % 8;
    while len > 0 {
        *ptr = val;
        ptr = (ptr as u64 + 1) as *mut u8;
        len -= 1;
    }
}
