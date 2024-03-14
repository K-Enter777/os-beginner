use crate::bitfield::BitField;

/// ディスクリプタの種類を（ビットフィールドとして）表す。
/// 内部は [SystemSegmentType] もしくは [CodeDataSegmentType] が入っている。
///
/// ディスクリプタには2種類あり、ディスクリプタの6バイト目、\[4:0\] ビットで表現されている。
/// 4ビット目（S: Descriptor type bit）がどちらのディスクリプタであるかを表し、
///
/// 4ビット目が立っている場合は、
/// メモリセグメントに関する情報を含むコード・データセグメントを表している。
///
/// 4ビット目が立っていない場合、
/// メモリセグメント以外の情報を含んでいるシステムセグメントであることを表す。
/// \[3:0\] ビットが具体的にどのようなタイプであるかを表す。
#[derive(Debug, Clone, Copy)]
pub(crate) struct DescriptorType(u8);

impl DescriptorType {
    #![allow(unused)]

    /// コード・データセグメントのディスクリプタタイプを作る。
    pub(crate) fn code_data_segment(accesed: bool, rw: bool, dc: bool, executable: bool) -> Self {
        let mut data = 0;
        data.set_bit(4, true);
        data.set_bits(
            ..4,
            CodeDataSegmentType::new(accesed, rw, dc, executable).into(),
        );
        Self(data)
    }

    /// システムセグメントのディスクリプタタイプを作る。
    pub(crate) fn system_segment(r#type: SystemSegmentType) -> Self {
        let mut data = 0;
        data.set_bits(..4, r#type as u8);
        Self(data)
    }

    /// ディスクリプタがコード・データセグメントを表しているかどうかを得る。
    pub(crate) fn is_code_data_segment(&self) -> bool {
        self.0.get_bit(4)
    }

    /// ディスクリプタがシステムセグメントを表しているかどうかを表す。
    pub(crate) fn is_system_segment(&self) -> bool {
        !self.is_code_data_segment()
    }

    /// 4ビット目のチェックを行わずにコード・データセグメントのタイプを得る。
    fn as_code_data_segment_unchecked(&self) -> CodeDataSegmentType {
        self.0.get_bits(..4).into()
    }

    /// 4ビット目のチェックを行わずにシステムセグメントのタイプを得る。
    fn as_system_segment_unchecked(&self) -> SystemSegmentType {
        self.0.get_bits(..4).into()
    }

    /// ディスクリプタがコード・データセグメントを表していればそう解釈して返す。
    pub(crate) fn as_code_data_segment(&self) -> Option<CodeDataSegmentType> {
        if self.is_code_data_segment() {
            Some(self.as_code_data_segment_unchecked())
        } else {
            None
        }
    }

    /// ディスクリプタがシステムセグメントを表していればそう解釈して返す。
    pub(crate) fn as_system_segment(&self) -> Option<SystemSegmentType> {
        if self.is_system_segment() {
            Some(self.as_system_segment_unchecked())
        } else {
            None
        }
    }

    /// ディスクリプタがコード・データセグメント、システムセグメントを表しているかどうかと、
    /// その中身を返す。
    pub(crate) fn get(&self) -> DescriptorTypeEnum {
        if self.is_code_data_segment() {
            DescriptorTypeEnum::CodeData(self.as_code_data_segment_unchecked())
        } else {
            DescriptorTypeEnum::System(self.as_system_segment_unchecked())
        }
    }
}

impl From<u8> for DescriptorType {
    fn from(value: u8) -> Self {
        if value.get_bit(4) {
            Self::code_data_segment(
                value.get_bit(0),
                value.get_bit(1),
                value.get_bit(2),
                value.get_bit(3),
            )
        } else {
            Self::system_segment(value.get_bits(..4).into())
        }
    }
}

impl From<DescriptorType> for u8 {
    fn from(value: DescriptorType) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DescriptorTypeEnum {
    System(SystemSegmentType),
    CodeData(CodeDataSegmentType),
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub(crate) enum SystemSegmentType {
    // system セグメント、ゲートディスクリプタ
    Upper8Byte = 0,
    LDT = 2,
    TSSAvailable = 9,
    TSSBusy = 11,
    CallGate = 12,
    InterruptGate = 14,
    TrapGate = 15,
}

impl From<u8> for SystemSegmentType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Upper8Byte,
            2 => Self::LDT,
            9 => Self::TSSAvailable,
            11 => Self::TSSBusy,
            12 => Self::CallGate,
            14 => Self::InterruptGate,
            15 => Self::TrapGate,
            _ => panic!("cannot convert {} to DescriptorType", value),
        }
    }
}

/// コーデ・データセグメントを表す。
///
/// 詳細は以下を参照。
///
/// 0ビット目（A: Accesed bit）は、そのセグメントに CPU がアクセスした場合に1にセットする。
/// つまり、読み込み専用のときは 0 に設定しておかなければならない。
///
/// 1ビット目（RW: Readable/Writable bit）は、
/// そのセグメントがコードセグメントか、データセグメントかで意味が異なる。
///
/// コードセグメントであれば、ビットが立っていないときき読み取りが許可されておらず、
/// 立っているときは読み取りが許可される。
/// 書き込みは常に禁止である。
///
/// データセグメントであれば、ビットが立っていないとき書き込みが許可されておらず、
/// 立っているとき書き込みが許可される。
/// 読み込みは常に許可されている。
///
/// 2ビット目（DC: Direction/Conforming bit）も、
/// コードセグメントかデータセグメントかで意味が異なる。
///
/// コードセグメントであれば、
/// ビットが立っていないとき通常のリングプロテクションがかかり、
/// 自身と同じリングレベルからしか実行できない。
/// ビットが立っているときは、自身と同じリングレベルのもの、
/// もしくはそれ以下のリングレベルから far jump で実行できる。
///
/// データセグメントであれば、
/// ビットが立っているときセグメントが下方伸長（スタックなどで使われる）であることを表す。
///
/// 3ビット目（E: Executable bit）はコードセグメント、データセグメントのどちらであるかを表す。
/// ビットが立っていないときはデータセグメントであり、
/// 立っていればコードセグメントを表す。
#[derive(Debug, Clone, Copy)]
pub(crate) struct CodeDataSegmentType(u8);

impl CodeDataSegmentType {
    #![allow(unused)]

    /// それぞれのビットが立っているかどうかを指定して、
    /// コード・データセグメントを作る。
    pub(crate) fn new(accessed: bool, rw: bool, dc: bool, executable: bool) -> Self {
        let mut data = 0;
        data.set_bit(0, accessed);
        data.set_bit(1, rw);
        data.set_bit(2, dc);
        data.set_bit(3, executable);
        Self(data)
    }

    /// CPU によってアクセスされたかどうかを得る。
    pub(crate) fn is_accessed(&self) -> bool {
        self.0.get_bit(0)
    }

    /// 読み込み/書き込み可能であるかを得る。
    pub(crate) fn is_readable_writable(&self) -> bool {
        self.0.get_bit(1)
    }

    /// 下方伸長/適応型であるかを得る。
    pub(crate) fn is_growdown_conforming(&self) -> bool {
        self.0.get_bit(2)
    }

    /// コードセグメントかどうかを得る。
    pub(crate) fn is_executable(&self) -> bool {
        self.0.get_bit(3)
    }
}

impl From<u8> for CodeDataSegmentType {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl From<CodeDataSegmentType> for u8 {
    fn from(value: CodeDataSegmentType) -> Self {
        value.0
    }
}
