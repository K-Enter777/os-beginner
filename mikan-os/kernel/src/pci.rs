#![allow(unused)]

use alloc::vec::{self, Vec};
use core::{
    cell::RefCell,
    fmt::{self, Display, LowerHex},
    ptr::addr_of_mut,
};

use crate::{
    asmfunc::{io_in_32, io_out_32},
    bitfield::BitField,
    error::{self, Result},
    make_error,
    sync::RwLock,
};

/// CONFIG_ADDRESS レジスタの IO ポートアドレス
const CONFIG_ADDRESS: u16 = 0x0cf8;
/// CONFIG_DATA レジスタの IO ポートアドレス
const CONFIG_DATA: u16 = 0x0cfc;

/// PCI デバイスのクラスコード。
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassCode {
    base: u8,
    sub: u8,
    interface: u8,
}

impl ClassCode {
    /// ベースクラスが等しいかどうか。
    pub(crate) fn match_base(&self, b: u8) -> bool {
        b == self.base
    }

    /// ベースクラスとサブクラスが等しいかどうか。
    pub(crate) fn match_base_sub(&self, b: u8, s: u8) -> bool {
        self.match_base(b) && s == self.sub
    }

    /// ベース、サブ、インターフェースが等しいかどうか。
    pub(crate) fn r#match(&self, b: u8, s: u8, i: u8) -> bool {
        self.match_base_sub(b, s) && i == self.interface
    }
}

impl Display for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            ((self.base as u32) << 24) | ((self.sub as u32) << 16) | ((self.interface as u32) << 8)
        )
    }
}

impl LowerHex for ClassCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        LowerHex::fmt(
            &(((self.base as u32) << 24)
                | ((self.sub as u32) << 16)
                | ((self.interface as u32) << 8)),
            f,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Device {
    bus: u8,
    device: u8,
    function: u8,
    header_type: u8,
    class_code: ClassCode,
}

impl Device {
    pub(crate) fn new(
        bus: u8,
        device: u8,
        function: u8,
        header_type: u8,
        class_code: ClassCode,
    ) -> Self {
        Self {
            bus,
            device,
            function,
            header_type,
            class_code,
        }
    }

    pub(crate) fn bus(&self) -> u8 {
        self.bus
    }

    pub(crate) fn device(&self) -> u8 {
        self.device
    }

    pub(crate) fn function(&self) -> u8 {
        self.function
    }

    pub(crate) fn header_type(&self) -> u8 {
        self.header_type
    }

    pub(crate) fn class_code(&self) -> ClassCode {
        self.class_code
    }

    pub(crate) fn read_vendor_id(&self) -> u16 {
        read_vendor_id(self.bus, self.device, self.function)
    }

    pub(crate) fn read_conf_reg(&self, reg_addr: u8) -> u32 {
        write_address(make_address(self.bus, self.device, self.function, reg_addr));
        read_data()
    }

    pub(crate) fn write_conf_reg(&self, reg_addr: u8, value: u32) {
        write_address(make_address(self.bus, self.device, self.function, reg_addr));
        write_data(value);
    }

    pub(crate) fn read_bar(&self, bar_index: u32) -> Result<u64> {
        if bar_index >= 6 {
            return Err(make_error!(error::Code::IndexOutOfRange));
        }

        let addr = cals_bar_address(bar_index);
        let bar = self.read_conf_reg(addr);

        // 32 bit アドレス
        if !bar.get_bit(2) {
            return Ok(bar as u64);
        }

        // 64 bit アドレス
        if bar_index >= 5 {
            return Err(make_error!(error::Code::IndexOutOfRange));
        }

        let bar_upper = self.read_conf_reg(addr + 4);
        Ok(bar as u64 | (bar_upper as u64) << 32)
    }

    fn read_capability_header(&self, addr: u8) -> CapabilityHeader {
        CapabilityHeader {
            data: self.read_conf_reg(addr),
        }
    }

    fn configure_msi(
        &mut self,
        msg_addr: u32,
        msg_data: u32,
        num_vector_exponent: u32,
    ) -> Result<()> {
        let mut cap_addr = self.read_conf_reg(0x34) & 0xff;
        let mut msi_cap_addr = 0;
        let mut msix_cap_addr = 0;

        while cap_addr != 0 {
            let header = self.read_capability_header(cap_addr as u8);
            if header.bits().cap_id() == CAPABILITY_MSI as u32 {
                msi_cap_addr = cap_addr;
            } else if header.bits().cap_id() == CAPABILITY_MSIX as u32 {
                msix_cap_addr = cap_addr;
            }
            cap_addr = header.bits().next_ptr();
        }

        if msi_cap_addr != 0 {
            self.configure_msi_register(msi_cap_addr as u8, msg_addr, msg_data, num_vector_exponent)
        } else if msix_cap_addr != 0 {
            self.configure_msix_register(
                msix_cap_addr as u8,
                msg_addr,
                msg_data,
                num_vector_exponent,
            )
        } else {
            Err(make_error!(error::Code::NoPCIMSI))
        }
    }

    /// 指定された MSI ケーパビリティ構造を読み取る
    ///
    /// * `dev` - MSI ケーパビリティを読み込む PCI デバイス
    /// * `cap_addr` - MSI ケーパビリティレジスタのコンフィギュレーション空間アドレス
    fn read_msi_capability(&self, cap_addr: u8) -> MSICapability {
        let header = MSICapabilityHeader {
            data: self.read_conf_reg(cap_addr),
        };
        let msg_addr = self.read_conf_reg(cap_addr + 4);

        let (msg_upper_addr, msg_data_addr) = if header.bits().addr_64_capable() != 0 {
            (self.read_conf_reg(cap_addr + 8), cap_addr + 12)
        } else {
            (0, cap_addr + 8)
        };

        let msg_data = self.read_conf_reg(msg_data_addr);

        let (mask_bits, pending_bits) = if header.bits().per_vector_mask_capable() != 0 {
            (
                self.read_conf_reg(msg_data_addr + 4),
                self.read_conf_reg(msg_data_addr + 8),
            )
        } else {
            (0, 0)
        };

        MSICapability {
            header,
            msg_addr,
            msg_upper_addr,
            msg_data,
            mask_bits,
            pending_bits,
        }
    }

    fn write_msi_capability(&self, cap_addr: u8, msi_cap: &MSICapability) {
        let header = msi_cap.header;
        self.write_conf_reg(cap_addr, header.data());
        self.write_conf_reg(cap_addr + 4, msi_cap.msg_addr);

        let msg_data_addr = if header.bits().addr_64_capable() != 0 {
            self.write_conf_reg(cap_addr + 8, msi_cap.msg_upper_addr);
            cap_addr + 12
        } else {
            cap_addr + 8
        };

        self.write_conf_reg(msg_data_addr, msi_cap.msg_data);

        if header.bits().per_vector_mask_capable() != 0 {
            self.write_conf_reg(msg_data_addr + 4, msi_cap.mask_bits);
            self.write_conf_reg(msg_data_addr + 8, msi_cap.pending_bits);
        }
    }

    /// 指定された MSI レジスタを設定する。
    fn configure_msi_register(
        &self,
        cap_addr: u8,
        msg_addr: u32,
        msg_data: u32,
        num_vector_exponent: u32,
    ) -> Result<()> {
        let mut msi_cap = self.read_msi_capability(cap_addr);

        // なんか packed 構造体の要素への参照は UB（未定義動作）らしい
        let header = addr_of_mut!(msi_cap.header);
        let enable = if unsafe { *header }.bits().multi_msg_capable() <= num_vector_exponent {
            unsafe { *header }.bits().multi_msg_capable()
        } else {
            num_vector_exponent
        };

        // packed 構造体の参照を生ポインタから使おうとしても、それも UB っぽい
        // 挙動を見る限りはコピーが起きている（ポインタを見てもそうなっている）
        // そのため、できることは生ポインタに対して直接 `write_unaligned()` といメソッドで
        // 値を上書きすることだけ
        // （中身は `memcpy` で 1 バイトずつコピーしているっぽい）
        let mut old = unsafe { *header };
        old.bits_mut().set_multi_msg_enable(enable);
        old.bits_mut().set_msi_enable(1);
        unsafe { header.write_unaligned(old) };

        msi_cap.msg_addr = msg_addr;
        msi_cap.msg_data = msg_data;

        self.write_msi_capability(cap_addr, &msi_cap);
        Ok(())
    }

    fn configure_msix_register(
        &mut self,
        cap_addr: u8,
        msg_addr: u32,
        msg_data: u32,
        num_vector_exponent: u32,
    ) -> Result<()> {
        Err(make_error!(error::Code::NotImplemented))
    }

    pub(crate) fn configure_msi_fixed_destination(
        &mut self,
        apic_id: u8,
        trigger_mode: MSITriggerMode,
        delivery_mode: MSIDeliverMode,
        vector: u8,
        num_vector_exponent: u32,
    ) -> Result<()> {
        let msg_addr = 0xfee0_0000 | ((apic_id as u32) << 12);
        let mut msg_data = ((delivery_mode as u32) << 8) | vector as u32;
        if trigger_mode == MSITriggerMode::Level {
            msg_data |= 0xc000;
        }
        self.configure_msi(msg_addr, msg_data, num_vector_exponent)
    }
}

/// CONFIG_ADDRESS に指定された整数を書き込む。
pub(crate) fn write_address(address: u32) {
    unsafe {
        io_out_32(CONFIG_ADDRESS, address);
    }
}

/// CONFIG_DATA に指定された整数を書き込む。
pub(crate) fn write_data(value: u32) {
    unsafe {
        io_out_32(CONFIG_DATA, value);
    }
}

/// CONFIG_DATA から 32 ビット整数を読み込む。
pub(crate) fn read_data() -> u32 {
    unsafe { io_in_32(CONFIG_DATA) }
}

/// ベンダ ID レジスタを読み取る（全ヘッダタイプ共通）。
pub(crate) fn read_vendor_id(bus: u8, device: u8, function: u8) -> u16 {
    write_address(make_address(bus, device, function, 0x00));
    read_data() as u16
}

/// デバイス ID レジスタを読み取る（全ヘッダタイプ共通）。
pub(crate) fn read_device_id(bus: u8, device: u8, function: u8) -> u16 {
    write_address(make_address(bus, device, function, 0x00));
    (read_data() >> 16) as u16
}

/// ヘッダタイプレジスタを読み取る（全ヘッダタイプ共通）。
pub(crate) fn read_header_type(bus: u8, device: u8, function: u8) -> u8 {
    write_address(make_address(bus, device, function, 0x0c));
    (read_data() >> 16) as u8
}

/// クラスコード・レジスタを読み取る（全ヘッダタイプ共通）。
pub(crate) fn read_class_code(bus: u8, device: u8, function: u8) -> ClassCode {
    write_address(make_address(bus, device, function, 0x08));
    let reg = read_data();
    ClassCode {
        base: (reg >> 24) as u8,
        sub: (reg >> 16) as u8,
        interface: (reg >> 8) as u8,
    }
}

/// バス番号レジスタを読み取る（ヘッダタイプ 1 用）。
///
/// 返される 32 ビット整数の構造は次の通り
/// - 23:16 : サブオーディネイトバス番号
/// - 15:8  : セカンダリバス番号
/// - 7:0   : リビジョン番号
pub(crate) fn read_bus_numbers(bus: u8, device: u8, function: u8) -> u32 {
    write_address(make_address(bus, device, function, 0x18));
    read_data()
}

/// 単一ファンクションの場合に真を返す。
fn is_single_function_device(header_type: u8) -> bool {
    !header_type.get_bit(7)
}

/// [DEVICES] の配列長。
const DEVICE_MAX_LEN: usize = 32;
/// [scan_all_bus] により発見された PCI デバイスの一覧。
pub(crate) static DEVICES: RwLock<Vec<Device>> = RwLock::new(Vec::new());

const fn cals_bar_address(bar_index: u32) -> u8 {
    0x10 + 4 * bar_index as u8
}

/// PCI デバイスをすべて探索し、[DEVICES] に格納する。
///
/// バス 0 から再帰的に PCI デバイスを探索し、[DEVICES] の先頭から詰めて書き込む。
/// 発見したデバイスの数を [NUM_DEVICES] に設定する。
pub(crate) fn scan_all_bus() -> Result<()> {
    let mut num_device = 0;

    let header_type = read_header_type(0, 0, 0);
    if is_single_function_device(header_type) {
        return scan_bus(0);
    }

    for function in 0..8 {
        if read_vendor_id(0, 0, function) == 0xffff {
            continue;
        }
        scan_bus(function)?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
#[repr(packed)]
pub(crate) struct CapabilityHeaderBits {
    cap_id: u8,
    next_ptr: u8,
    cap: u16,
}

impl CapabilityHeaderBits {
    pub(crate) const fn cap_id(&self) -> u32 {
        self.cap_id as u32
    }

    pub(crate) const fn next_ptr(&self) -> u32 {
        self.next_ptr as u32
    }

    pub(crate) const fn cap(&self) -> u32 {
        self.cap as u32
    }
}

/// PCI ケーパビリティレジスタの共通ヘッダ
#[repr(packed)]
pub(crate) union CapabilityHeader {
    data: u32,
    bits: CapabilityHeaderBits,
}

impl CapabilityHeader {
    fn bits(&self) -> &CapabilityHeaderBits {
        unsafe { &self.bits }
    }

    fn bits_mut(&mut self) -> &mut CapabilityHeaderBits {
        unsafe { &mut self.bits }
    }
}

impl CapabilityHeader {
    pub(crate) const fn new(data: u32) -> Self {
        Self { data }
    }
}

const CAPABILITY_MSI: u8 = 0x05;
const CAPABILITY_MSIX: u8 = 0x11;

#[derive(Clone, Copy)]
#[repr(packed)]
pub(crate) struct MSICapabilityHeaderBits {
    cap_id: u8,
    next_ptr: u8,
    etc_1: u8,
    etc_2: u8,
}

impl MSICapabilityHeaderBits {
    pub(crate) fn cap_id(&self) -> u32 {
        self.cap_id as u32
    }

    pub(crate) fn next_ptr(&self) -> u32 {
        self.next_ptr as u32
    }

    pub(crate) fn msi_enable(&self) -> u32 {
        (self.etc_1 & 0x01) as u32
    }

    pub(crate) fn set_msi_enable(&mut self, value: u32) {
        self.etc_1 = (self.etc_1 & !0x01) | ((value as u8) & 0x01);
    }

    pub(crate) fn multi_msg_capable(&self) -> u32 {
        ((self.etc_1 >> 1) & 0x07) as u32
    }

    pub(crate) fn multi_msg_enable(&self) -> u32 {
        ((self.etc_1 >> 4) & 0x07) as u32
    }

    pub(crate) fn set_multi_msg_enable(&mut self, value: u32) {
        self.etc_1.set_bits(4..7, value as u8);
    }

    pub(crate) fn addr_64_capable(&self) -> u32 {
        self.etc_1.get_bit(7).into()
    }

    pub(crate) fn per_vector_mask_capable(&self) -> u32 {
        self.etc_2.get_bit(0).into()
    }
}

#[derive(Clone, Copy)]
pub(crate) union MSICapabilityHeader {
    data: u32,
    bits: MSICapabilityHeaderBits,
}

impl MSICapabilityHeader {
    fn data(&self) -> u32 {
        unsafe { self.data }
    }

    fn bits(&self) -> &MSICapabilityHeaderBits {
        unsafe { &self.bits }
    }

    fn bits_mut(&mut self) -> &mut MSICapabilityHeaderBits {
        unsafe { &mut self.bits }
    }
}

/// MSI ケーパビリティ構造は 64 ビットサポートの有無などで亜種が沢山ある。
/// この構造体は各亜種に対応するために最大の亜種に合わせてメンバを定義してある。
#[repr(packed)]
pub(crate) struct MSICapability {
    header: MSICapabilityHeader,
    msg_addr: u32,
    msg_upper_addr: u32,
    msg_data: u32,
    mask_bits: u32,
    pending_bits: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MSITriggerMode {
    Edge = 0,
    Level = 1,
}

pub(crate) enum MSIDeliverMode {
    Fixed = 0b000,
    LowestPriority = 0b001,
    SMI = 0b010,
    NMI = 0b100,
    INIT = 0b101,
    ExtINT = 0b111,
}

pub(crate) fn initialize_pci() {}

fn make_address(bus: u8, device: u8, function: u8, reg_addr: u8) -> u32 {
    let shl = |x: u8, bits: u32| (x as u32) << bits;

    shl(1, 31) | shl(bus, 16) | shl(device, 11) | shl(function, 8) | (reg_addr & 0xfc) as u32
}

fn add_device(device: Device) -> Result<()> {
    let mut devices = DEVICES.write();

    devices.push(device);
    Ok(())
}

/// 指定のファンクションを devices に追加する。
/// もし PCI-PCI ブリッジなら、セカンダリバスに対し [scan_bus] を実行する。
fn scan_function(bus: u8, device: u8, function: u8) -> Result<()> {
    let class_code = read_class_code(bus, device, function);
    let header_type = read_header_type(bus, device, function);
    let dev = Device::new(bus, device, function, header_type, class_code);

    add_device(dev)?;

    // PCI-PCI ブリッジの場合
    if class_code.match_base_sub(0x06, 0x04) {
        let bus_number = read_bus_numbers(bus, device, function);
        let secondary_bus = bus_number.get_bits(8..16);
        return scan_bus(secondary_bus as u8);
    }

    Ok(())
}

/// 指定のデバイス番号の各ファンクションをスキャンする。
/// 有効なファンクションを見つけたら [scan_function] を実行する。
fn scan_device(bus: u8, device: u8) -> Result<()> {
    scan_function(bus, device, 0)?;
    if is_single_function_device(read_header_type(bus, device, 0)) {
        return Ok(());
    }

    for function in 1..8 {
        if read_vendor_id(bus, device, function) == 0xffff {
            continue;
        }
        scan_function(bus, device, function)?;
    }
    Ok(())
}

/// 指定のバス番号の各デバイスをスキャンする。
/// 有効なデバイスを見つけたら [scan_device] を実行する。
fn scan_bus(bus: u8) -> Result<()> {
    for device in 0..DEVICE_MAX_LEN as u8 {
        if read_vendor_id(bus, device, 0) == 0xffff {
            continue;
        }
        scan_device(bus, device)?;
    }
    Ok(())
}
