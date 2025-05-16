// src/rules/mapping.rs
use std::collections::HashSet;

// Maps file extensions (lowercase) or exact filenames to rule stems
// This function determines which static rule file (e.g., "rust.org")
// might be relevant based on a file characteristic found in the project.
pub fn map_characteristic_to_rule_stem(characteristic: &str) -> Option<&'static str> {
    match characteristic {
        // File Extensions (matched lowercase)
        "rs" => Some("rust"),
        "rb" => Some("ruby"),
        "c" | "h" => Some("c"),
        "cpp" | "hpp" => Some("cpp"),
        "go" => Some("go"),
        "js" | "cjs" | "mjs" | "jsx" => Some("javascript"),
        "ts" | "tsx" => Some("typescript"),
        "php" => Some("php"),
        "org" | "md" => Some("documentation"), // Org/Markdown files trigger documentation rules
        "json" | "yaml" | "yml" | "toml" | "xml" => Some("config_file"), // Config files trigger config rules
        "rake" => Some("rakefile"), // Files with .rake extension

        // Filenames (exact match - case sensitivity respected here)
        "Rakefile" => Some("rakefile"),
        "Gemfile" => Some("ruby"), // Gemfile also implies ruby rules
        // Add more specific filename mappings here if needed (e.g., "Cargo.toml" -> "rust"?)
        _ => None, // No known rule stem for this characteristic
    }
}

// Defines the default set of rule stems that are always included
// by default, unless explicitly excluded in the user's config.
pub fn get_default_rule_stems() -> HashSet<&'static str> {
    // Use an array literal and convert to HashSet for clarity
    ["general", "guidelines", "documentation"]
        .iter()
        .cloned()
        .collect()
}
