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
