use core::arch::global_asm;

extern "C" {
    pub(crate) fn io_out_32(addr: u16, data: u32);
    pub(crate) fn io_in_32(addr: u16) -> u32;
    pub(crate) fn get_cs() -> u16;
    pub(crate) fn load_idt(limit: u16, offset: u64);
}

global_asm! { r#"
.global io_out_32
io_out_32:
    mov dx, di
    mov eax, esi
    out dx, eax
    ret

.global io_in_32
io_in_32:
    mov dx, di
    in eax, dx
    ret

.global get_cs
get_cs:
    xor eax, eax
    mov ax, cs
    ret

.global load_idt
load_idt: 
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di
    mov [rsp + 2], rsi
    lidt [rsp]
    mov rsp, rbp
    pop rbp
    ret
"# }
