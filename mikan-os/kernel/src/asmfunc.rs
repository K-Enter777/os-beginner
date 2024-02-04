use core::arch::global_asm;

extern "C" {
    pub(crate) fn io_out_32(addr: u16, data: u32);
    pub(crate) fn io_in_32(addr: u16) -> u32;
    pub(crate) fn get_cs() -> u16;
    pub(crate) fn load_idt(limit: u16, offset: u64);
    pub(crate) fn load_gdt(limit: u16, offset: u64);
    pub(crate) fn set_ds_all(value: u16);
    pub(crate) fn set_cs_ss(cs: u16, ss: u16);
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

.global load_gdt
load_gdt:
    push rbp
    mov rbp, rsp
    sub rsp, 10
    mov [rsp], di # limit
    mov [rsp + 2], rsi # offset
    lgdt [rsp]
    mov rsp, rbp
    pop rbp
    ret

.global set_ds_all
set_ds_all:
    mov ds, di
    mov es, di
    mov fs, di
    mov gs, di
    ret

.global set_cs_ss
set_cs_ss:
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
"# }

// 引数は System-V ABI では rdi, rsi に渡される
// それらはこの関数内部でいじっていないので、引数はないものとしも問題ない
extern "sysv64" {
    #[allow(unused)]
    pub fn kernel_entry();
}

// カーネルの本当のエントリーポイント
// スタック領域の変更を行う
// 何故か `KERNEL_MAIN_STACK + 1024 * 1024` が メモリアクセスにコンパイルされてしまうため、
// `lea` 命令に変更してある
global_asm! { r#"
.global kernel_entry
kernel_entry:
    lea rsp, KERNEL_MAIN_STACK + 1024 * 1024
    call kernel_main_new_stack
.fin:
    hlt
    jmp .fin
"#
}
