use core::arch::global_asm;

extern "C" {
    pub(crate) fn io_out_32(addr: u16, data: u32);
    pub(crate) fn io_in_32(addr: u16) -> u32;
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
"# }
