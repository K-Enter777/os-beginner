use crate::{
    bitfield::BitField,
    x86_descriptor::{DescriptorType, SystemSegmentType},
};

#[repr(packed)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct InterruptDescriptorAttribute {
    etc_1: u8,
    etc_2: u8,
}

impl InterruptDescriptorAttribute {
    #![allow(unused)]
    pub(crate) const fn const_default() -> Self {
        Self { etc_1: 0, etc_2: 0 }
    }

    pub(crate) fn new(
        r#type: SystemSegmentType,
        descriptor_privilege_level: u8,
        present: bool,
    ) -> Self {
        let mut etc_2 = 0;
        etc_2.set_bits(..4, DescriptorType::system_segment(r#type).into());
        etc_2.set_bits(5..7, descriptor_privilege_level);
        etc_2.set_bit(7, present);
        Self { etc_1: 0, etc_2 }
    }

    pub(crate) fn interrupt_stack_table(&self) -> u8 {
        self.etc_1 & 0x07
    }

    pub(crate) fn r#type(&self) -> DescriptorType {
        DescriptorType::from(self.etc_2 & 0x0f)
    }

    pub(crate) fn descriptor_privilege_level(&self) -> u8 {
        (self.etc_2 >> 5) & 0x03
    }

    pub(crate) fn present(&self) -> bool {
        (self.etc_2 >> 7) & 0x01 == 1
    }
}

impl Default for InterruptDescriptorAttribute {
    fn default() -> Self {
        Self::const_default()
    }
}

#[repr(packed)]
#[derive(Default, Clone, Copy)]
pub(crate) struct InterruptDescriptor {
    offset_low: u16,
    segment_selector: u16,
    attr: InterruptDescriptorAttribute,
    offset_middle: u16,
    offset_high: u32,
    _reserved: u32,
}

impl InterruptDescriptor {
    pub(crate) const fn const_default() -> Self {
        Self {
            offset_low: 0,
            segment_selector: 0,
            attr: InterruptDescriptorAttribute::const_default(),
            offset_middle: 0,
            offset_high: 0,
            _reserved: 0,
        }
    }

    pub(crate) fn set_idt_entry(
        &mut self,
        attr: InterruptDescriptorAttribute,
        entry: unsafe extern "C" fn(),
        segment_selector: u16,
    ) {
        let offset = entry as *const fn() as u64;
        self.attr = attr;
        self.offset_low = offset as u16;
        self.offset_middle = (offset >> 16) as u16;
        self.offset_high = (offset >> 32) as u32;
        self.segment_selector = segment_selector;
    }
}

pub(crate) fn notify_end_of_interrupt() {
    let end_of_interrupt = 0xfee0_00b0 as *mut u32;
    unsafe {
        end_of_interrupt.write_volatile(0);
    }
}

pub(crate) enum InterruptVector {
    XHCI = 0x40,
}

pub(crate) struct InterruptFrame {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

impl InterruptFrame {
    #![allow(unused)]

    pub(crate) fn rip(&self) -> u64 {
        self.rip
    }

    pub(crate) fn cs(&self) -> u64 {
        self.cs
    }

    pub(crate) fn rflags(&self) -> u64 {
        self.rflags
    }

    pub(crate) fn rsp(&self) -> u64 {
        self.rsp
    }

    pub(crate) fn ss(&self) -> u64 {
        self.ss
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub(crate) enum MessageType {
    InteruptXHCI,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Message {
    r#type: MessageType,
}

impl Message {
    pub(crate) fn new(r#type: MessageType) -> Self {
        Self { r#type }
    }

    pub(crate) fn r#type(&self) -> MessageType {
        self.r#type
    }
}
