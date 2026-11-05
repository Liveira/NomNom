use std::path::Path;

/// Messages are shown to the user as they are, so they are plain sentences.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConvertError {
    #[error("Couldn't read this file as {0}.")]
    NotReadable(&'static str),
    #[error("{0}")]
    BadOutput(&'static str),
    #[error("{0}")]
    Io(String),
    #[error("{0} isn't installed.")]
    MissingTool(&'static str),
    #[error("{0}")]
    Tool(String),
    #[error("Stopped.")]
    Cancelled,
    #[error("{0}")]
    Other(String),
}

impl ConvertError {
    pub fn io(path: &Path, e: std::io::Error) -> Self {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        ConvertError::Io(match e.kind() {
            std::io::ErrorKind::NotFound => format!("{name} couldn't be found."),
            std::io::ErrorKind::PermissionDenied => format!("NOMNOM isn't allowed to use {name}."),
            _ => format!("Problem with {name}: {e}"),
        })
    }
}
