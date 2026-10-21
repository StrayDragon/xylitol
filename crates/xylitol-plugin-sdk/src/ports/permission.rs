#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XyPermissionVerdict {
    Allow,
    Deny { reason: String },
}

impl XyPermissionVerdict {
    pub fn is_allowed(&self) -> bool {
        matches!(self, XyPermissionVerdict::Allow)
    }

    pub fn deny_reason(&self) -> Option<&str> {
        match self {
            XyPermissionVerdict::Deny { reason } => Some(reason.as_str()),
            XyPermissionVerdict::Allow => None,
        }
    }
}

pub trait XyPermission: Send + Sync {
    fn check_read(&self, path: &str) -> XyPermissionVerdict;
    fn check_write(&self, path: &str) -> XyPermissionVerdict;
    fn check_network(&self, domain: &str) -> XyPermissionVerdict;
    fn check_process(&self, path: &str) -> XyPermissionVerdict;
}
