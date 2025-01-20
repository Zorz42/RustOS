.globl usertrap
.globl uservec
.align 4
uservec:
    csrw sscratch, a0

    li a0, 7 << 30

    // save the registers.
    sd ra, 0(a0)
    sd sp, 8(a0)
    sd gp, 16(a0)
    sd tp, 24(a0)
    sd t0, 32(a0)
    sd t1, 40(a0)
    sd t2, 48(a0)
    sd s0, 56(a0)
    sd s1, 64(a0)
    //sd a0, 72(a0) // a0 is saved in sscratch
    sd a1, 80(a0)
    sd a2, 88(a0)
    sd a3, 96(a0)
    sd a4, 104(a0)
    sd a5, 112(a0)
    sd a6, 120(a0)
    sd a7, 128(a0)
    sd s2, 136(a0)
    sd s3, 144(a0)
    sd s4, 152(a0)
    sd s5, 160(a0)
    sd s6, 168(a0)
    sd s7, 176(a0)
    sd s8, 184(a0)
    sd s9, 192(a0)
    sd s10, 200(a0)
    sd s11, 208(a0)
    sd t3, 216(a0)
    sd t4, 224(a0)
    sd t5, 232(a0)
    sd t6, 240(a0)

    // save the sepc
    csrr t0, sepc
    sd t0, 248(a0)

    // load the kernel hartid
    ld tp, 256(a0)

    csrr t0, sscratch
    sd t0, 72(a0)

    li sp, 128 * 1024 * 1024 + 0x80000000
    li a0, 1024 * 64
    mul a0, a0, tp
    sub sp, sp, a0

    # will not return
    call usertrap

.globl jump_to_user
.align 4
jump_to_user:
    li a0, 7 << 30
    // load jump address from 248(a0)
    ld t0, 248(a0)
    csrw sepc, t0

    sd tp, 256(a0) // store to kernel_hartid

    // load all registers from 0(a0) to 240(a0)
    ld ra, 0(a0)
    ld sp, 8(a0)
    ld gp, 16(a0)
    ld tp, 24(a0)
    ld t0, 32(a0)
    ld t1, 40(a0)
    ld t2, 48(a0)
    ld s0, 56(a0)
    ld s1, 64(a0)
    //ld a0, 72(a0) // this one gets loaded last
    ld a1, 80(a0)
    ld a2, 88(a0)
    ld a3, 96(a0)
    ld a4, 104(a0)
    ld a5, 112(a0)
    ld a6, 120(a0)
    ld a7, 128(a0)
    ld s2, 136(a0)
    ld s3, 144(a0)
    ld s4, 152(a0)
    ld s5, 160(a0)
    ld s6, 168(a0)
    ld s7, 176(a0)
    ld s8, 184(a0)
    ld s9, 192(a0)
    ld s10, 200(a0)
    ld s11, 208(a0)
    ld t3, 216(a0)
    ld t4, 224(a0)
    ld t5, 232(a0)
    ld t6, 240(a0)

    ld a0, 72(a0)

    // jump to user mode
    sret
