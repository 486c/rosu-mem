use std::{num::ParseIntError, string::FromUtf8Error};

#[derive(thiserror::Error, Debug)]
pub enum ProcessError {
    #[error("process not found")]
    ProcessNotFound,
    #[error("executable path not found")]
    ExecutablePathNotFound,
    #[error("not enough permissions to run, please run as admin/sudo")]
    NotEnoughPermissions,
    #[error("io error")]
    IoError(#[from] std::io::Error),
    #[error("failed to convert bytes to string")]
    FromUtf8Error(#[from] FromUtf8Error),
    #[error("failed to convert type")]
    ConvertionError,
    #[error("trying to read bad address, addr: {0:X}, len: {1:X}")]
    BadAddress(usize, usize),
    #[error("cannot find signature: {0}")]
    SignatureNotFound(String),
    #[error("os error `{0}`")]
    #[cfg(target_os = "linux")]
    OsError(#[from] nix::errno::Errno),
    #[cfg(target_os = "windows")]
    OsError(#[from] windows::core::Error),
    #[error("failed to convert address to usize")]
    AddressConvertError,
}

impl From<std::num::ParseIntError> for ProcessError {
    fn from(_: std::num::ParseIntError) -> Self {
        Self::ConvertionError
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseSignatureError {
    #[error("invalid string length `{0}`")]
    InvalidLength(usize),
    #[error("failed to parse integer")]
    InvalidInt(#[from] ParseIntError),
}
