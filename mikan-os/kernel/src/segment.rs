use core::mem::size_of;

use crate::{
    asmfunc::load_gdt,
    bitfield::BitField,
    x86_descriptor::{DescriptorType, DescriptorTypeEnum, SystemSegmentType},
};

static mut GDT: [SegmentDescriptor; 3] = [SegmentDescriptor::default(); 3];

pub(crate) fn setup_segments() {
    unsafe {
        GDT[1] = SegmentDescriptor::code_segment(0, 0xfffff, false, true, false, 0);
        GDT[2] = SegmentDescriptor::data_segment(0, 0xfffff, false, true, true, 0);
        load_gdt(
            (size_of::<SegmentDescriptor>() * GDT.len()) as u16,
            GDT.as_ptr() as u64,
        );
    }
}

/// セグメントディスクリプタを表す。
#[derive(Debug, Default, Clone, Copy)]
#[repr(packed)]
pub(crate) struct SegmentDescriptor {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    etc_1: u8,
    etc_2: u8,
    base_high: u8,
}

impl SegmentDescriptor {
    #![allow(unused)]

    /// システムセグメントを表すディスクリプタを作る。
    pub(crate) fn system_segment(
        base: u32,
        limit: u32,
        r#type: SystemSegmentType,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(..5, DescriptorType::system_segment(r#type).into());
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(6, true); // default_operation_size
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    /// コードセグメントを表すディスクリプタを作る。
    pub(crate) fn code_segment(
        base: u32,
        limit: u32,
        accesed: bool,
        readable: bool,
        conforming: bool,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(
            ..5,
            DescriptorType::code_data_segment(accesed, readable, conforming, true).into(),
        );
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(4, false); // available
        etc_2.set_bit(5, true); // long mode
        etc_2.set_bit(6, false); // default_operation_size should be 0 when long_mode = 1
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    /// データセグメントを表すディスクリプタを作る。
    pub(crate) fn data_segment(
        base: u32,
        limit: u32,
        accesed: bool,
        writable: bool,
        direction: bool,
        descriptor_privilege_level: u8,
    ) -> Self {
        let base_low = base.get_bits(..16) as u16;
        let base_middle = base.get_bits(16..24) as u8;
        let base_high = base.get_bits(24..32) as u8;
        let limit_low = limit.get_bits(..16) as u16;
        let limit_high = limit.get_bits(16..20) as u8;

        let mut etc_1 = 0;
        etc_1.set_bits(
            ..5,
            DescriptorType::code_data_segment(accesed, writable, direction, false).into(),
        );
        etc_1.set_bits(5..7, descriptor_privilege_level);
        etc_1.set_bit(7, true); // present

        let mut etc_2 = 0;
        etc_2.set_bits(..4, limit_high);
        etc_2.set_bit(4, false); // available
        etc_2.set_bit(5, false); // long mode should be 0
        etc_2.set_bit(6, true); // default_operation_size
        etc_2.set_bit(7, true); // granalarity

        Self {
            limit_low,
            base_low,
            base_middle,
            etc_1,
            etc_2,
            base_high,
        }
    }

    /// ヌルディスクリプタを作る。
    pub(crate) const fn default() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            etc_1: 0,
            etc_2: 0,
            base_high: 0,
        }
    }

    /// リミットを取得する。
    pub(crate) fn limit(&self) -> u32 {
        (self.etc_2.get_bits(4..) as u32) << 16 | self.limit_low as u32
    }

    /// ベースを取得する。
    pub(crate) fn base(&self) -> u32 {
        (self.base_high as u32) << 24 | (self.base_middle as u32) << 16 | self.base_low as u32
    }

    /// 種類を [DescriptorType] として取得する。
    pub(crate) fn type_raw(&self) -> DescriptorType {
        self.etc_1.get_bits(..5).into()
    }

    /// 種類を [DescriptorTypeEnum] として取得する。
    pub(crate) fn r#type(&self) -> DescriptorTypeEnum {
        self.type_raw().get()
    }

    /// ディスクリプタが表しているのがシステムセグメントであるかを取得する。
    pub(crate) fn is_system_segment(&self) -> bool {
        self.type_raw().is_system_segment()
    }

    /// ディスクリプタが表しているのがコード・データセグメントであるかを取得する。
    pub(crate) fn is_code_data_segment(&self) -> bool {
        self.type_raw().is_code_data_segment()
    }

    /// DPL を取得する。
    pub(crate) fn descriptor_privilege_level(&self) -> u8 {
        self.etc_1.get_bits(5..7)
    }

    /// 有効かどうかを取得する。
    pub(crate) fn present(&self) -> bool {
        self.etc_1.get_bit(7)
    }

    /// available（プログラムが利用可能）を取得する。
    pub(crate) fn available(&self) -> bool {
        self.etc_2.get_bit(4)
    }

    /// ロングモードが有効かどうかを取得する。
    pub(crate) fn long_mode(&self) -> bool {
        self.etc_2.get_bit(5)
    }

    /// Default ビットを取得する。
    pub(crate) fn default_operation_size(&self) -> bool {
        self.etc_2.get_bit(6)
    }

    /// Granuality ビットを取得する。
    pub(crate) fn granalarity(&self) -> bool {
        self.etc_2.get_bit(7)
    }
}
