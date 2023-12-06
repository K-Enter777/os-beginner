#![allow(unused)]

use core::fmt::{self, Display};

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Code {
    Success,
    Full,
    Empty,
    NoEnoughMemory,
    IndexOutOfRange,
    HostControllerNotHalted,
    InvalidSlotID,
    PortNotConnected,
    InvalidEndpointNumber,
    TransferRingNotSet,
    AlreadyAllocated,
    NotImplemented,
    InvalidDescriptor,
    BufferTooSmall,
    UnknownDevice,
    NoCorrespondingSetupStage,
    TransferFailed,
    InvalidPhase,
    UnknownXHCISpeedID,
    NoWaiter,
    NoPCIMSI,
    UnknownPixelFormat,
    NoSuchTask,
    InvalidFormat,
    FrameTooSmall,
    InvalidFile,
    IsDirectory,
    NoSuchEntry,
    FreeTypeError,
    EndpointNotInCharge,
    LastOfCode, // これは常に最後に配置する
}

impl Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "Success"),
            Self::Full => write!(f, "Full"),
            Self::Empty => write!(f, "Empty"),
            Self::NoEnoughMemory => write!(f, "NoEnoughMemory"),
            Self::IndexOutOfRange => write!(f, "IndexOutOfRange"),
            Self::HostControllerNotHalted => write!(f, "HostControllerNotHalted"),
            Self::InvalidSlotID => write!(f, "InvalidSlotID"),
            Self::PortNotConnected => write!(f, "PortNotConnected"),
            Self::InvalidEndpointNumber => write!(f, "InvalidEndpointNumber"),
            Self::TransferRingNotSet => write!(f, "TransferRingNotSet"),
            Self::AlreadyAllocated => write!(f, "AlreadyAllocated"),
            Self::NotImplemented => write!(f, "NotImplemented"),
            Self::InvalidDescriptor => write!(f, "InvalidDescriptor"),
            Self::BufferTooSmall => write!(f, "BufferTooSmall"),
            Self::UnknownDevice => write!(f, "UnknownDevice"),
            Self::NoCorrespondingSetupStage => write!(f, "NoCorrespondingSetupStage"),
            Self::TransferFailed => write!(f, "TransferFailed"),
            Self::InvalidPhase => write!(f, "InvalidPhase"),
            Self::UnknownXHCISpeedID => write!(f, "UnknownXHCISpeedID"),
            Self::NoWaiter => write!(f, "NoWaiter"),
            Self::NoPCIMSI => write!(f, "NoPCIMSI"),
            Self::UnknownPixelFormat => write!(f, "UnknownPixelFormat"),
            Self::NoSuchTask => write!(f, "NoSuchTask"),
            Self::InvalidFormat => write!(f, "InvalidFormat"),
            Self::FrameTooSmall => write!(f, "FrameTooSmall"),
            Self::InvalidFile => write!(f, "InvalidFile"),
            Self::IsDirectory => write!(f, "IsDirectory"),
            Self::NoSuchEntry => write!(f, "NoSuchEntry"),
            Self::FreeTypeError => write!(f, "FreeTypeError"),
            Self::EndpointNotInCharge => write!(f, "EndpointNotInCharge"),
            Self::LastOfCode => write!(f, "LastOfCode"),
        }
    }
}

pub(crate) struct Error {
    code: Code,
    line: u32,
    file: &'static str,
}

impl Error {
    pub(crate) const fn new(code: Code, file: &'static str, line: u32) -> Self {
        Self { code, line, file }
    }

    pub(crate) const fn cause(&self) -> Code {
        self.code
    }

    pub(crate) const fn file(&self) -> &str {
        self.file
    }

    pub(crate) const fn line(&self) -> u32 {
        self.line
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in {} at {}", self.code, self.file, self.line)
    }
}

#[macro_export]
macro_rules! make_error {
    ($code:expr) => {
        $crate::error::Error::new($code, file!(), line!())
    };
}

impl From<Error> for bool {
    fn from(value: Error) -> Self {
        value.code != Code::Success
    }
}

impl From<&Error> for bool {
    fn from(value: &Error) -> Self {
        value.code != Code::Success
    }
}

pub(crate) struct WithError<T> {
    value: T,
    error: Error,
}

impl<T> WithError<T> {
    pub(crate) fn new(value: T, error: Error) -> Self {
        Self { value, error }
    }
}
