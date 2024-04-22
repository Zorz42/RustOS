use core::arch::asm;

pub unsafe fn volatile_store_byte(ptr: *mut u8, value: u8) {
    asm!("mov [{}], {}", in(reg) ptr, in(reg_byte) value);
}

pub unsafe fn memcpy(src: *mut u8, dst: *mut u8, len: usize) {
    // check for alignment
    debug_assert_eq!(src as u64 % 8, 0);
    debug_assert_eq!(dst as u64 % 8, 0);
    debug_assert_eq!(len % 8, 0);
    
    // check for non-overlapping
    debug_assert!(src as u64 + len as u64 <= dst as u64 || src as u64 >= dst as u64 + len as u64);
    
    asm!("
    mov rax, {}
    mov rbx, {}    
    mov rcx, {}
    mov rdx, rax
    add rdx, rcx
    2:
    mov rcx, [rax]
    mov [rbx], rcx
    add rax, 8
    add rbx, 8
    cmp rax, rdx
    jne 2b
    
    ", in(reg) src, in(reg) dst, in(reg) len, options(preserves_flags, nostack));
}

pub unsafe fn memcpy_non_aligned(src: *mut u8, dst: *mut u8, len: usize) {
    // check for non-overlapping
    debug_assert!(src as u64 + len as u64 <= dst as u64 || src as u64 >= dst as u64 + len as u64);
    
    asm!("
    mov rax, {}
    mov rbx, {}
    mov rcx, {}
    mov rdx, rax
    add rdx, rcx
    2:
    mov cl, [rax]
    mov [rbx], cl
    add rax, 1
    add rbx, 1
    cmp rax, rdx
    jne 2b
    
    ", in(reg) src, in(reg) dst, in(reg) len, options(preserves_flags, nostack));
}

pub unsafe fn memset_int64(ptr: *mut u8, val: u64, len: usize) {
    debug_assert_eq!(len % 8, 0);
    debug_assert_eq!(len as u64 % 8, 0);
    
    asm!("
    mov rax, {}
    mov rbx, rax
    add rbx, {}
    2:
    mov [rax], {}
    add rax, 8
    cmp rax, rbx
    jne 2b
    
    ", in(reg) ptr, in(reg) len, in(reg) val, options(preserves_flags, nostack));
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