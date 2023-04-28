pub enum ConnectionType {
    Upload,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            ConnectionType::Upload => write!(f, "Upload"),
        }
    }
}

impl Clone for ConnectionType {
    fn clone(&self) -> Self {
        match self {
            ConnectionType::Upload => ConnectionType::Upload,
        }
    }
}
