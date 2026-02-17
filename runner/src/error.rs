use std::fmt;

#[derive(Debug)]
pub enum RunError {
    FileRead(String, std::io::Error),
    NoRequestBlock(usize),
    ParseFailed(String),
    Transport(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::FileRead(path, e) => write!(f, "Error reading {}: {}", path, e),
            RunError::NoRequestBlock(line) => {
                write!(f, "No request block found at line {}", line)
            }
            RunError::ParseFailed(msg) => write!(f, "Parse error: {}", msg),
            RunError::Transport(msg) => write!(f, "Transport error: {}", msg),
        }
    }
}

impl std::error::Error for RunError {}
