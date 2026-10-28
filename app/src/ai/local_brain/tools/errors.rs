use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ToolArgumentError {
    InvalidSchema(String),
    MissingField(&'static str),
    InvalidFieldType(&'static str),
}

impl Display for ToolArgumentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolArgumentError::InvalidSchema(msg) => write!(f, "Invalid schema: {}", msg),
            ToolArgumentError::MissingField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            ToolArgumentError::InvalidFieldType(field) => {
                write!(f, "Invalid field type for field: {}", field)
            }
        }
    }
}

impl std::error::Error for ToolArgumentError {}

#[derive(Debug)]
pub enum ToolEncodingError {
    ConversionFailed(String),
}

impl Display for ToolEncodingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolEncodingError::ConversionFailed(msg) => write!(f, "Tool encoding failed: {}", msg),
        }
    }
}

impl std::error::Error for ToolEncodingError {}

#[derive(Debug)]
pub enum ToolResultRenderError {
    SerializationFailed(serde_json::Error),
    ExtractionFailed(String),
}

impl Display for ToolResultRenderError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolResultRenderError::SerializationFailed(e) => {
                write!(f, "Serialization failed: {}", e)
            }
            ToolResultRenderError::ExtractionFailed(msg) => {
                write!(f, "Result extraction failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for ToolResultRenderError {}

impl From<serde_json::Error> for ToolResultRenderError {
    fn from(e: serde_json::Error) -> Self {
        ToolResultRenderError::SerializationFailed(e)
    }
}
