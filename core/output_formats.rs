use crate::error::{AppError, Result};
use once_cell::sync::Lazy;
#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))]
pub struct FileContextInfo {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))]
pub struct SourceRepresentation {
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub files: Option<Vec<FileContextInfo>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub chunks: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))]
pub struct ChunkInfo {
    pub current_part: usize,
    pub total_parts: usize,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))]
pub struct ChunkFile {
    pub files: Vec<FileContextInfo>,
    pub chunk_info: ChunkInfo,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde_support", derive(Deserialize))]
pub struct AiReadmeText {
    pub intro: String,
    pub key_sections_header: String,
    pub project_name_desc: String,
    pub project_root_desc: String,
    pub system_info_desc: String,
    pub meta_desc: String,
    pub docs_desc: String,
    pub tree_desc: String,
    pub source_files_desc: String,
    pub source_chunks_desc: String,
    pub source_missing_desc: String,
    pub rules_desc: String,
    pub rules_missing_desc: String,
    pub timestamp_desc: String,
}
#[derive(Debug, Default)]
#[cfg_attr(feature = "serde_support", derive(Deserialize))]
pub struct BuiltinIgnores {
    #[serde(default)]
    pub common: Vec<String>,
    #[serde(default)]
    pub tree: Vec<String>,
    #[serde(default)]
    pub source: Vec<String>,
    #[serde(default)]
    pub docs: Vec<String>,
}

static PREDEFINED_PROMPTS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    // Corrected path: "../data/"
    let yaml_content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../data/prompts.yaml"));
    serde_yml::from_str(yaml_content).expect("Failed to parse embedded data/prompts.yaml")
});
static AI_README_TEXT: Lazy<AiReadmeText> = Lazy::new(|| {
    // Corrected path: "../data/"
    let yaml_content = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/ai_readme.yaml"
    ));
    serde_yml::from_str(yaml_content).expect("Failed to parse embedded data/ai_readme.yaml")
});
static BUILTIN_IGNORE_PATTERNS: Lazy<BuiltinIgnores> = Lazy::new(|| {
    // Corrected path: "../data/"
    let yaml_content = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../data/builtin_ignores.yaml"
    ));
    serde_yml::from_str(yaml_content).expect("Failed to parse embedded data/builtin_ignores.yaml")
});

pub fn get_predefined_prompts() -> &'static HashMap<String, String> {
    &PREDEFINED_PROMPTS
}
pub fn get_ai_readme_text() -> &'static AiReadmeText {
    &AI_README_TEXT
}
pub fn get_builtin_ignore_patterns() -> &'static BuiltinIgnores {
    &BUILTIN_IGNORE_PATTERNS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextType {
    Prompt,
}
pub fn get_predefined_text(name: &str, text_type: TextType) -> Result<String> {
    let map = match text_type {
        TextType::Prompt => get_predefined_prompts(),
    };
    match map.get(name) {
        Some(text) => Ok(text.clone()),
        None => {
            let type_str = match text_type {
                TextType::Prompt => "prompt",
            };
            Err(AppError::Config(format!(
                "Predefined {} name \\\"{}\\\" specified in config not found.",
                type_str, name
            )))
        }
    }
}

#[cfg(feature = "serde_support")]
pub fn serialize_context_to_json<T: Serialize>(
    context: &T,
    pretty: bool,
) -> Result<String, AppError> {
    if pretty {
        serde_json::to_string_pretty(context).map_err(AppError::JsonSerialize)
    } else {
        serde_json::to_string(context).map_err(AppError::JsonSerialize)
    }
}

#[cfg(feature = "serde_support")]
pub fn serialize_context_to_yaml<T: Serialize>(context: &T) -> Result<String, AppError> {
    serde_yml::to_string(context).map_err(AppError::YamlError)
}

#[cfg(feature = "serde_support")]
pub fn serialize_context_to_xml<T: Serialize>(
    context: &T,
    root_name: &str,
    _pretty: bool, // Mark pretty as unused for now
) -> Result<String, AppError> {
    // Use the simpler helper function which avoids manual Serializer creation
    quick_xml::se::to_string_with_root(root_name, context)
        .map_err(|e| AppError::XmlSerialize(e.to_string()))

    // --- Keep the manual code commented out in case we need pretty printing later ---
    /*
    use quick_xml::se::Serializer;
    use quick_xml::Writer;
    use std::io::Cursor;

    let mut buf = Vec::new();
    // Create the writer wrapping the buffer
    let mut writer = if pretty {
        Writer::new_with_indent(Cursor::new(&mut buf), b' ', 4)
    } else {
        Writer::new(Cursor::new(&mut buf))
    };

    // This block is likely causing the trait bound issue
    { // Scope might help, but maybe not needed with to_string_with_root
        let mut ser = Serializer::with_root(&mut writer, Some(root_name))?;
        context.serialize(ser)?;
    } // End of scope for ser

    // Retrieve the buffer content
    // NOTE: This might need adjustment depending on how `writer`'s state is managed
    // after `ser` is dropped or consumed by `serialize`.
    // Let's assume `buf` directly holds the data for now if not using the helper.
    // let final_buf = writer.into_inner().into_inner().to_owned();
    String::from_utf8(buf).map_err(|e| AppError::XmlSerialize(e.to_string()))
    */
}
