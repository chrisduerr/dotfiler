use std::{error, fmt, io};
use handlebars;
use toml;

#[derive(Debug)]
pub enum DotfilerError {
    Message(String),
    IoError(io::Error),
    TomlError(toml::de::Error),
    TomlSerializerError(toml::ser::Error),
    TemplateRenderError(Box<handlebars::TemplateRenderError>),
}

impl fmt::Display for DotfilerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DotfilerError::Message(ref err) => write!(f, "{}", err),
            DotfilerError::IoError(ref err) => write!(f, "IO error: {}", err),
            DotfilerError::TomlError(ref err) => write!(f, "Toml error: {}", err),
            DotfilerError::TemplateRenderError(ref err) => write!(f, "Template error: {}", err),
            DotfilerError::TomlSerializerError(ref err) => write!(f, "Serializer error: {}", err),
        }
    }
}

impl error::Error for DotfilerError {
    fn description(&self) -> &str {
        match *self {
            DotfilerError::Message(ref err) => err,
            DotfilerError::IoError(ref err) => err.description(),
            DotfilerError::TomlError(ref err) => err.description(),
            DotfilerError::TemplateRenderError(ref err) => err.description(),
            DotfilerError::TomlSerializerError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DotfilerError::Message(ref err) => None,
            DotfilerError::IoError(ref err) => Some(err),
            DotfilerError::TomlError(ref err) => Some(err),
            DotfilerError::TemplateRenderError(ref err) => Some(err),
            DotfilerError::TomlSerializerError(ref err) => Some(err),
        }
    }
}

impl From<toml::de::Error> for DotfilerError {
    fn from(err: toml::de::Error) -> DotfilerError {
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
        DotfilerError::TemplateRenderError(Box::new(err))
    }
}

impl From<toml::ser::Error> for DotfilerError {
    fn from(err: toml::ser::Error) -> DotfilerError {
        DotfilerError::TomlSerializerError(err)
    }
}

impl From<String> for DotfilerError {
    fn from(err: String) -> DotfilerError {
        DotfilerError::Message(err)
    }
}
