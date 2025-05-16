use crate::config::{self, Config, ResolvedRules};
use crate::error::Result;
use crate::gather::{self, TreeNode}; // Corrected: Use gather::TreeNode
// Removed unused import: use crate::output_formats::AiReadmeText;
use crate::output_formats::{FileContextInfo, SourceRepresentation, get_ai_readme_text};
use crate::system::SystemInfo;
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use log;
#[cfg(feature = "serde_support")]
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
#[cfg_attr(feature = "serde_support", serde(rename_all = "camelCase"))] // Or snake_case
pub struct ProjectContext {
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none", rename = "aiReadme")
    )]
    pub ai_readme: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub project_name: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub project_root: Option<String>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub system_info: Option<SystemInfo>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub meta: Option<HashMap<String, String>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub docs: Option<Vec<FileContextInfo>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub tree: Option<Vec<TreeNode>>, // This uses gather::TreeNode now
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub source: Option<SourceRepresentation>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "IndexMap::is_empty")
    )]
    pub rules: IndexMap<String, Vec<String>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub prompts: Option<HashMap<String, String>>,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub generation_timestamp: Option<DateTime<Utc>>,

    // Internal data not serialized
    #[cfg_attr(feature = "serde_support", serde(skip))]
    pub resolved_rules_debug: Option<ResolvedRules>, // Keep for debug command
}

impl ProjectContext {
    pub fn build(
        project_root_path: &Path,
        config: &Config,
        tree_structure: Option<Vec<TreeNode>>,
        project_characteristics: &HashSet<String>,
    ) -> Result<Self> {
        log::debug!("Building project context skeleton...");

        let sys_info = if config.output.include_system_info {
            log::trace!("Gathering system info...");
            Some(crate::system::gather_system_info()?)
        } else {
            log::trace!("System info collection disabled.");
            None
        };

        let meta_map = if config.meta.enabled && !config.meta.custom_meta.is_empty() {
            log::trace!("Preparing metadata...");
            Some(config.meta.custom_meta.clone())
        } else {
            log::trace!("Metadata disabled or empty.");
            None
        };

        log::trace!("Resolving rules...");
        let resolved_rules =
            config::resolve_rules(&config.rules, project_root_path, project_characteristics)?;
        let resolved_rules_debug_info = resolved_rules.clone();
        log::trace!("Rules resolved.");

        log::trace!("Resolving prompts...");
        let prompts_section = match config::resolve_prompts(&config.prompts, project_root_path) {
            Ok(map) if !map.is_empty() => Some(map),
            _ => None,
        };
        log::trace!("Prompts resolved.");

        let mut context = ProjectContext {
            ai_readme: None, // Will be populated later
            project_name: if config.output.include_project_name {
                Some(config.get_effective_project_name(project_root_path))
            } else {
                None
            },
            project_root: if config.output.include_project_root {
                Some(project_root_path.to_string_lossy().to_string())
            } else {
                None
            },
            system_info: sys_info,
            meta: meta_map,
            docs: None, // Populated by add_docs
            tree: if config.tree.enabled {
                tree_structure
            } else {
                None
            },
            source: None, // Populated by add_files or add_chunk_paths
            rules: resolved_rules.rulesets,
            prompts: prompts_section,
            generation_timestamp: if config.output.include_timestamp {
                Some(Utc::now())
            } else {
                None
            },
            resolved_rules_debug: Some(resolved_rules_debug_info),
        };

        context.populate_ai_readme(config); // Populate initial readme

        log::debug!("Context skeleton built successfully.");
        Ok(context)
    }

    fn create_file_context_list(
        files_info: Vec<gather::FileInfo>,
        project_root: &Path,
    ) -> Vec<FileContextInfo> {
        files_info
            .into_iter()
            .map(|finfo| FileContextInfo {
                path: pathdiff::diff_paths(&finfo.path, project_root)
                    .unwrap_or_else(|| finfo.path.clone()) // Fallback to absolute if diff fails
                    .to_string_lossy()
                    .to_string(),
                content: finfo.content,
            })
            .collect()
    }

    pub fn add_files(
        mut self,
        source_files_info: Vec<gather::FileInfo>,
        project_root: &Path,
        config: &Config, // Needed to repopulate readme
    ) -> Self {
        if config.source.enabled && !source_files_info.is_empty() {
            log::debug!(
                "Adding {} source files inline to context.",
                source_files_info.len()
            );
            self.source = Some(SourceRepresentation {
                files: Some(Self::create_file_context_list(
                    source_files_info,
                    project_root,
                )),
                chunks: None,
            });
        } else if config.source.enabled {
            log::debug!("No source files provided or found to add inline.");
            self.source = None; // Explicitly set to None if enabled but no files
        } else {
            log::debug!("Source section disabled, not adding files.");
            self.source = None;
        }
        self.populate_ai_readme(config); // Repopulate after potentially changing source
        self
    }

    pub fn add_docs(
        mut self,
        docs_files_info: Vec<gather::FileInfo>,
        project_root: &Path,
        config: &Config, // Needed to repopulate readme
    ) -> Self {
        if config.docs.enabled && !docs_files_info.is_empty() {
            log::debug!(
                "Adding {} documentation files to context.",
                docs_files_info.len()
            );
            self.docs = Some(Self::create_file_context_list(
                docs_files_info,
                project_root,
            ));
        } else if config.docs.enabled {
            log::debug!("No documentation files provided or found.");
            self.docs = None;
        } else {
            log::debug!("Docs section disabled, not adding files.");
            self.docs = None;
        }
        self.populate_ai_readme(config); // Repopulate after potentially changing docs
        self
    }

    pub fn add_chunk_paths(
        mut self,
        chunk_paths: Vec<PathBuf>,
        save_dir: &Path,
        config: &Config, // Needed to repopulate readme
    ) -> Self {
        if config.source.enabled && !chunk_paths.is_empty() {
            log::debug!(
                "Adding {} chunk file references to context.",
                chunk_paths.len()
            );
            let relative_chunk_paths = chunk_paths
                .into_iter()
                .map(|p| {
                    // Try to make path relative to save_dir, fallback to original path string
                    pathdiff::diff_paths(&p, save_dir)
                        .map(|rel_p| rel_p.to_string_lossy().to_string())
                        .unwrap_or_else(|| p.to_string_lossy().to_string())
                })
                .collect();
            self.source = Some(SourceRepresentation {
                files: None,
                chunks: Some(relative_chunk_paths),
            });
        } else if config.source.enabled {
            log::debug!("No chunk paths provided.");
            // If chunking was expected but produced no files, source might be empty
            self.source = None;
        } else {
            log::debug!("Source section disabled, not adding chunk paths.");
            self.source = None;
        }
        self.populate_ai_readme(config); // Repopulate after potentially changing source representation
        self
    }

    pub fn populate_ai_readme(&mut self, config: &Config) {
        let readme_template = get_ai_readme_text();
        let mut parts: Vec<&str> = Vec::new();
        parts.push(&readme_template.intro);

        let mut details: Vec<&str> = Vec::new();
        if self.project_name.is_some() {
            details.push(&readme_template.project_name_desc);
        }
        if self.project_root.is_some() {
            details.push(&readme_template.project_root_desc);
        }
        if self.system_info.is_some() {
            details.push(&readme_template.system_info_desc);
        }
        if self.meta.is_some() {
            details.push(&readme_template.meta_desc);
        }
        if self.docs.is_some() {
            details.push(&readme_template.docs_desc);
        }
        if self.tree.is_some() {
            details.push(&readme_template.tree_desc);
        }

        if let Some(source_repr) = &self.source {
            if source_repr.files.is_some() {
                details.push(&readme_template.source_files_desc);
            } else if source_repr.chunks.is_some() {
                details.push(&readme_template.source_chunks_desc);
            } else {
                // Should not happen if source is Some, but handle defensively
                details.push(&readme_template.source_missing_desc);
            }
        } else if config.source.enabled {
            // Source enabled but no files/chunks added yet or found
            details.push(&readme_template.source_missing_desc);
        }
        // No else needed if source is disabled

        if !self.rules.is_empty() {
            details.push(&readme_template.rules_desc);
        } else if config.rules.enabled {
            // Rules enabled but none resolved/found
            details.push(&readme_template.rules_missing_desc);
        }
        // No else needed if rules are disabled

        if self.generation_timestamp.is_some() {
            details.push(&readme_template.timestamp_desc);
        }

        if !details.is_empty() {
            parts.push(&readme_template.key_sections_header);
            parts.extend(details);
            self.ai_readme = Some(parts.join("\\n"));
        } else {
            // Fallback if somehow no sections are included
            self.ai_readme = Some(readme_template.intro.clone());
        }
        log::trace!("AI Readme populated.");
    }
}
