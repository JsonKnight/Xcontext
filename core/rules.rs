use crate::error::{AppError as Error, Result};
use log;
use rust_embed::RustEmbed; // Added use statement
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

pub mod mapping; // Keep this declaration

#[derive(RustEmbed)]
#[folder = "../data/rules/"] // Corrected path relative to core crate root
#[prefix = "rules/"] // Keep prefix for access path
struct StaticRuleAssets;

pub fn get_static_rule_content(rule_stem: &str) -> Result<String> {
    let file_path = format!("rules/{}.org", rule_stem);
    log::trace!("Attempting to get embedded static rule: {}", file_path);
    // Now StaticRuleAssets::get should work because RustEmbed trait is in scope
    let asset = StaticRuleAssets::get(&file_path).ok_or_else(|| {
        Error::RuleLoading(format!(
            "Static rule file not found in embed: {}",
            file_path
        ))
    })?;
    let content = std::str::from_utf8(asset.data.as_ref()).map_err(|e| {
        Error::RuleLoading(format!("UTF-8 error in embedded rule {}: {}", file_path, e))
    })?;
    Ok(content.to_string())
}

pub fn detect_project_characteristics(project_root: &Path) -> Result<HashSet<String>> {
    let mut characteristics = HashSet::new();
    log::debug!(
        "Detecting project characteristics in: {}",
        project_root.display()
    );
    let walker = WalkDir::new(project_root).follow_links(false); //.max_depth(3); // Consider limiting depth

    for entry_result in walker {
        match entry_result {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    let path = entry.path();
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        match filename {
                            // Specific filenames implying characteristics
                            "Rakefile" | "Gemfile" | "Cargo.toml" | "package.json"
                            | "composer.json" | "go.mod" | "Makefile" => {
                                log::trace!("Detected characteristic (filename): {}", filename);
                                characteristics.insert(filename.to_string());
                            }
                            _ => {}
                        }
                    }
                    if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
                        let lower_ext = extension.to_lowercase();
                        if characteristics.insert(lower_ext.clone()) {
                            log::trace!("Detected characteristic (extension): {}", lower_ext);
                        }
                        // Special handling if needed, e.g., .rake extension
                        if extension == "rake" {
                            if characteristics.insert("rake".to_string()) {
                                log::trace!(
                                    "Detected characteristic (extension): .rake maps to 'rake'"
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Error accessing path during characteristic detection: {} (at {})",
                    e,
                    e.path()
                        .map_or_else(|| "unknown path".into(), |p| p.display().to_string())
                );
                // Decide whether to continue or return error
                // For characteristics detection, usually best to continue
            }
        }
    }
    log::debug!("Detected characteristics: {:?}", characteristics);
    Ok(characteristics)
}

// Removed the inline `pub mod mapping { ... }` block that started around line 84
// The `pub mod mapping;` declaration at the top is sufficient.
