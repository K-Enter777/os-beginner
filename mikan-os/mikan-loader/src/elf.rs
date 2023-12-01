#![allow(unused)]

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Elf64Ehdr {
    pub(crate) ident: [u8; 16],
    pub(crate) r#type: u16,
    pub(crate) machine: u16,
    pub(crate) version: u32,
    pub(crate) entry: usize,
    pub(crate) phoff: u64,
    pub(crate) shoff: u64,
    pub(crate) flags: u32,
    pub(crate) ehsize: u16,
    pub(crate) phentsize: u16,
    pub(crate) phnum: u16,
    pub(crate) shentsize: u16,
    pub(crate) shnum: u16,
    pub(crate) shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Elf64Phdr {
    pub(crate) r#type: u32,
    pub(crate) flags: u32,
    pub(crate) offset: u64,
    pub(crate) vaddr: usize,
    pub(crate) paddr: usize,
    pub(crate) filesz: u64,
    pub(crate) memsz: u64,
    pub(crate) align: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) enum ProgramType {
    Null = 0,
    Load = 1,
    Dynamic = 2,
    Interp = 3,
    Note = 4,
    Shlib = 5,
    Phdr = 6,
    Tls = 7,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Elf64Dyn {
    tag: i64,
    val: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) enum DT {
    Null = 0,
    Rela = 7,
    Relasz = 8,
    Relaent = 9,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Elf64Rela {
    pub(crate) offset: u64,
    pub(crate) info: u32,
    pub(crate) addend: i32,
}
