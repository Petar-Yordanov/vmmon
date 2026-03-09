use std::{error::Error, fmt};

pub type Result<T> = std::result::Result<T, VmmonError>;

#[derive(Debug)]
pub enum VmmonError {
    Config(String),
    Io(std::io::Error),
    Kvm(kvm_ioctls::Error),
    Elf(String),
    Boot(String),
    Unsupported(String),
}

impl VmmonError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn elf(msg: impl Into<String>) -> Self {
        Self::Elf(msg.into())
    }

    pub fn boot(msg: impl Into<String>) -> Self {
        Self::Boot(msg.into())
    }

    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }
}

impl fmt::Display for VmmonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmmonError::Config(s) => write!(f, "config error: {s}"),
            VmmonError::Io(e) => write!(f, "io error: {e}"),
            VmmonError::Kvm(e) => write!(f, "kvm error: {e}"),
            VmmonError::Elf(s) => write!(f, "elf error: {s}"),
            VmmonError::Boot(s) => write!(f, "boot error: {s}"),
            VmmonError::Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}

impl Error for VmmonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            VmmonError::Io(e) => Some(e),
            VmmonError::Kvm(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for VmmonError {
    fn from(e: std::io::Error) -> Self {
        VmmonError::Io(e)
    }
}

impl From<kvm_ioctls::Error> for VmmonError {
    fn from(e: kvm_ioctls::Error) -> Self {
        VmmonError::Kvm(e)
    }
}
