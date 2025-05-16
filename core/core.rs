pub mod chunking;
pub mod config;
pub mod context;
pub mod error;
pub mod gather;
pub mod output_formats;
pub mod rules;
pub mod system;

pub use config::{Config, MetaConfig, PromptsConfig, ResolvedRules, RulesConfig};
pub use context::ProjectContext;
pub use error::{AppError, Result};
pub use gather::{FileInfo, TreeNode, gather_files_and_tree}; // Ensure TreeNode is re-exported
pub use output_formats::{
    AiReadmeText, BuiltinIgnores, ChunkFile, ChunkInfo, FileContextInfo, SourceRepresentation,
    TextType, get_ai_readme_text, get_builtin_ignore_patterns, get_predefined_text,
};
pub use rules::{detect_project_characteristics, get_static_rule_content};
pub use system::{SystemInfo, gather_system_info};
