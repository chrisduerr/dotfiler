use std::{error, fmt, io};
use handlebars;
use rusqlite;
use toml::de;

#[derive(Debug)]
pub enum DotfilerError {
    IoError(io::Error),
    TomlError(de::Error),
    RusqliteError(rusqlite::Error),
    TemplateRenderError(handlebars::TemplateRenderError),
}

impl fmt::Display for DotfilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DotfilerError::IoError(ref err) => write!(f, "IO error: {}", err),
            DotfilerError::TomlError(ref err) => write!(f, "Toml error: {}", err),
            DotfilerError::RusqliteError(ref err) => write!(f, "Rusqlite error: {}", err),
            DotfilerError::TemplateRenderError(ref err) => write!(f, "Template error: {}", err),
        }
    }
}

impl error::Error for DotfilerError {
    fn description(&self) -> &str {
        match *self {
            DotfilerError::IoError(ref err) => err.description(),
            DotfilerError::TomlError(ref err) => err.description(),
            DotfilerError::RusqliteError(ref err) => err.description(),
            DotfilerError::TemplateRenderError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DotfilerError::IoError(ref err) => Some(err),
            DotfilerError::TomlError(ref err) => Some(err),
            DotfilerError::RusqliteError(ref err) => Some(err),
            DotfilerError::TemplateRenderError(ref err) => Some(err),
        }
    }
}

impl From<de::Error> for DotfilerError {
    fn from(err: de::Error) -> DotfilerError {
        DotfilerError::TomlError(err)
    }
}

impl From<io::Error> for DotfilerError {
    fn from(err: io::Error) -> DotfilerError {
        DotfilerError::IoError(err)
    }
}

impl From<handlebars::TemplateRenderError> for DotfilerError {
    fn from(err: handlebars::TemplateRenderError) -> DotfilerError {
        DotfilerError::TemplateRenderError(err)
    }
}

impl From<rusqlite::Error> for DotfilerError {
    fn from(err: rusqlite::Error) -> DotfilerError {
        DotfilerError::RusqliteError(err)
    }
}
