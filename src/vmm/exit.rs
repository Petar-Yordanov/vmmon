use std::fmt;

#[derive(Debug, Clone)]
pub enum VmExitReason {
    Hlt,
    Shutdown,
    InternalError,
    Unhandled(String),
}

#[derive(Debug, Clone)]
pub struct VmExitInfo {
    pub reason: VmExitReason,
}

impl VmExitInfo {
    pub fn hlt() -> Self {
        Self {
            reason: VmExitReason::Hlt,
        }
    }

    pub fn shutdown() -> Self {
        Self {
            reason: VmExitReason::Shutdown,
        }
    }
}

impl fmt::Display for VmExitReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmExitReason::Hlt => write!(f, "hlt"),
            VmExitReason::Shutdown => write!(f, "shutdown"),
            VmExitReason::InternalError => write!(f, "internal-error"),
            VmExitReason::Unhandled(msg) => write!(f, "unhandled: {msg}"),
        }
    }
}

impl fmt::Display for VmExitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.reason)
    }
}
