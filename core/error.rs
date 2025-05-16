use std::path::PathBuf;
use thiserror::Error;

pub type Result<T, E = AppError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum AppError {
    #[error("Configuration Error: {0}")]
    Config(String),

    #[error("TOML Parsing Error: {0}")]
    TomlParse(String),

    #[error("TOML Serialization Error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON Serialization Error: {0}")]
    JsonSerialize(#[from] serde_json::Error),

    #[error("YAML Parsing/Serialization Error: {0}")]
    YamlError(#[from] serde_yml::Error),

    #[error("XML Serialization/Deserialization Error: {0}")]
    XmlSerialize(String),

    #[error("Filesystem Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File Read Error: Path '{path}', Error: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("File Write Error: Path '{path}', Error: {source}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Directory Creation Error: Path '{path}', Error: {source}")]
    DirCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("WalkDir Error: {0}")]
    WalkDir(String),

    #[error("Ignore Error: {0}")]
    Ignore(#[from] ignore::Error),

    #[error("Glob Pattern Error: {0}")]
    Glob(String),

    #[error("Chunking Error: {0}")]
    Chunking(String),

    #[error("System Info Error: {0}")]
    SystemInfo(String),

    #[error("Invalid Argument: {0}")]
    InvalidArgument(String),

    #[error("TikToken Error: {0}")]
    TikToken(String),

    #[error("Rule Loading Error: {0}")]
    RuleLoading(String),

    #[error("Data Loading Error: {0}")]
    DataLoading(String),

    #[error("Duration Parsing Error: {0}")]
    DurationParse(String),
}

#[cfg(feature = "serde_support")]
impl From<quick_xml::se::SeError> for AppError {
    fn from(err: quick_xml::se::SeError) -> Self {
        AppError::XmlSerialize(err.to_string())
    }
}
#[cfg(feature = "serde_support")]
impl From<quick_xml::DeError> for AppError {
    fn from(err: quick_xml::DeError) -> Self {
        AppError::XmlSerialize(err.to_string())
    }
}

impl From<globset::Error> for AppError {
    fn from(err: globset::Error) -> Self {
        AppError::Glob(format!("Globset error: {}", err))
    }
}

impl From<walkdir::Error> for AppError {
    fn from(err: walkdir::Error) -> Self {
        AppError::WalkDir(err.to_string())
    }
}

impl From<std::str::Utf8Error> for AppError {
    fn from(err: std::str::Utf8Error) -> Self {
        AppError::DataLoading(format!("UTF-8 decoding error: {}", err))
    }
}

impl From<parse_duration::parse::Error> for AppError {
    fn from(err: parse_duration::parse::Error) -> Self {
        AppError::DurationParse(err.to_string())
    }
}
