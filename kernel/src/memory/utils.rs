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