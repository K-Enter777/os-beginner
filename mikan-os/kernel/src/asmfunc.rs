use core::arch::global_asm;

pub(crate) fn io_out_32(addr: u16, data: u32) {
    unsafe { io_out_32_unsafe(addr, data) }
}

pub(crate) fn io_in_32(addr: u16) -> u32 {
    unsafe { io_in_32_unsafe(addr) }
}

pub(crate) fn get_cs() -> u16 {
    unsafe { get_cs_unsafe() }
}

pub(crate) fn load_idt(limit: u16, offset: u64) {
    unsafe { load_idt_unsafe(limit, offset) }
}

pub(crate) fn load_gdt(limit: u16, offset: u64) {
    unsafe { load_gdt_unsafe(limit, offset) }
}

pub(crate) fn set_ds_all(value: u16) {
    unsafe { set_ds_all_unsafe(value) }
}

pub(crate) fn set_cs_ss(cs: u16, ss: u16) {
    unsafe { set_cs_ss_unsafe(cs, ss) }
}

pub(crate) fn set_cr3(value: u64) {
    unsafe { set_cr3_unsafe(value) }
}

extern "C" {
    fn io_out_32_unsafe(addr: u16, data: u32);
    fn io_in_32_unsafe(addr: u16) -> u32;
    fn get_cs_unsafe() -> u16;
    fn load_idt_unsafe(limit: u16, offset: u64);
    fn load_gdt_unsafe(limit: u16, offset: u64);
    fn set_ds_all_unsafe(value: u16);
    fn set_cs_ss_unsafe(cs: u16, ss: u16);
    fn set_cr3_unsafe(value: u64);
}

global_asm! { r#"
.global io_out_32_unsafe
io_out_32_unsafe:
    mov dx, di
    mov eax, esi
    out dx, eax
    ret

.global io_in_32_unsafe
io_in_32_unsafe:
    mov dx, di
    in eax, dx
    ret

.global get_cs_unsafe
get_cs_unsafe:
    xor eax, eax
    mov ax, cs
    ret

.global load_idt_unsafe
load_idt_unsafe:
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di
    mov [rsp + 2], rsi
    lidt [rsp]
    mov rsp, rbp
    pop rbp
    ret

.global load_gdt_unsafe
load_gdt_unsafe:
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di # limit
    mov [rsp + 2], rsi # offset
    lgdt [rsp]
    mov rsp, rbp
    pop rbp
    ret

.global set_ds_all_unsafe
set_ds_all_unsafe:
    mov ds, di
    mov es, di
    mov fs, di
    mov gs, di
    ret

.global set_cs_ss_unsafe
set_cs_ss_unsafe:
    push rbp
    mov rbp, rsp
    mov ss, si
    lea rax, .next
    push rdi    # CS
    push rax    # RIP
    retfq
.next:
    mov rsp, rbp
    pop rbp
    ret

.global set_cr3_unsafe
set_cr3_unsafe:
    mov cr3, rdi
    ret
"# }
