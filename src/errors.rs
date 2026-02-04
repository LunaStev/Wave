use std::fmt;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CliError {
    Usage(String),

    // std
    StdAlreadyInstalled { path: PathBuf },
    ExternalToolMissing(&'static str),
    CommandFailed(String),
    HomeNotSet,

    // io
    Io(std::io::Error),
}

impl CliError {
    pub fn usage(msg: impl Into<String>) -> Self {
        CliError::Usage(msg.into())
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CliError::Usage(msg) => write!(f, "Error: {}", msg),
            CliError::StdAlreadyInstalled { path } => {
                write!(f, "Error: std already installed at '{}'", path.display())
            }
            CliError::ExternalToolMissing(t) => write!(f, "Error: required tool not found: {}", t),
            CliError::CommandFailed(msg) => write!(f, "Error: command failed: {}", msg),
            CliError::HomeNotSet => write!(f, "Error: HOME environment variable not set"),
            CliError::Io(e) => write!(f, "IO Error: {}", e),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}