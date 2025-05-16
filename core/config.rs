use crate::error::{AppError, Result};
use crate::rules::{self, mapping as rules_mapping};
use indexmap::IndexMap;
use log;
use parse_duration::parse;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const DEFAULT_CONFIG_DIR: &str = ".xtools/xcontext";
pub const DEFAULT_CONFIG_FILENAME: &str = "xcontext.toml";
pub const DEFAULT_CACHE_DIR: &str = ".xtools/xcontext/cache";
pub const DEFAULT_WATCH_DELAY: &str = "300ms";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub common_filters: CommonFiltersConfig,
    #[serde(default)]
    pub meta: MetaConfig,
    #[serde(default)]
    pub docs: DocsConfig,
    #[serde(default)]
    pub tree: TreeConfig,
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub rules: RulesConfig,
    #[serde(default)]
    pub prompts: PromptsConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub save: SaveConfig,
    #[serde(default)]
    pub watch: WatchConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default = "default_true")]
    pub use_gitignore: bool,
    #[serde(default = "default_true")]
    pub enable_builtin_ignore: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct CommonFiltersConfig {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MetaConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten, default)]
    pub custom_meta: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DocsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub use_gitignore: IgnoreSetting,
    #[serde(default)]
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TreeConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub use_gitignore: IgnoreSetting,
    #[serde(default)]
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub use_gitignore: IgnoreSetting,
    #[serde(default)]
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RulesConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub import: Vec<PathBuf>,
    #[serde(flatten)]
    pub custom: IndexMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct PromptsConfig {
    #[serde(default)]
    pub import: Vec<PathBuf>, // Changed to PathBuf
    #[serde(flatten, default)]
    pub custom: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OutputConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_true")]
    pub json_minify: bool,
    #[serde(default = "default_false")]
    pub xml_pretty_print: bool, // Renamed for clarity, maps to !xml_minify
    #[serde(default = "default_true")]
    pub include_project_name: bool,
    #[serde(default = "default_true")]
    pub include_project_root: bool,
    #[serde(default = "default_true")]
    pub include_system_info: bool,
    #[serde(default = "default_true")]
    pub include_timestamp: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveConfig {
    #[serde(default = "default_save_dir_config")]
    pub output_dir: PathBuf,
    #[serde(default)]
    pub filename_base: Option<String>,
    #[serde(default)]
    pub extension: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WatchConfig {
    #[serde(default = "default_watch_delay_string")]
    pub delay: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IgnoreSetting {
    Inherit,
    True,
    False,
}

impl Default for IgnoreSetting {
    fn default() -> Self {
        IgnoreSetting::Inherit
    }
}

fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}
fn default_format() -> String {
    "json".to_string()
}
fn default_save_dir_config() -> PathBuf {
    PathBuf::from(DEFAULT_CACHE_DIR)
}
fn default_watch_delay_string() -> String {
    DEFAULT_WATCH_DELAY.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            common_filters: CommonFiltersConfig::default(),
            meta: MetaConfig::default(),
            docs: DocsConfig::default(),
            tree: TreeConfig::default(),
            source: SourceConfig::default(),
            rules: RulesConfig::default(),
            prompts: PromptsConfig::default(),
            output: OutputConfig::default(),
            save: SaveConfig::default(),
            watch: WatchConfig::default(),
        }
    }
}
impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            project_name: None,
            use_gitignore: default_true(),
            enable_builtin_ignore: default_true(),
        }
    }
}
impl Default for MetaConfig {
    fn default() -> Self {
        let mut custom_meta = HashMap::new();
        custom_meta.insert("author".to_string(), "json".to_string());
        Self {
            enabled: default_true(),
            custom_meta,
        }
    }
}
impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            use_gitignore: IgnoreSetting::default(),
            include: Some(Vec::new()),
            exclude: Some(Vec::new()),
        }
    }
}
impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            use_gitignore: IgnoreSetting::default(),
            include: Some(Vec::new()),
            exclude: Some(Vec::new()),
        }
    }
}
impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            enabled: default_true(),
            use_gitignore: IgnoreSetting::default(),
            include: Some(Vec::new()),
            exclude: Some(Vec::new()),
        }
    }
}
impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            include: Vec::new(),
            exclude: Vec::new(),
            import: Vec::new(),
            custom: IndexMap::new(),
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            json_minify: default_true(),
            xml_pretty_print: default_false(),
            include_project_name: default_true(),
            include_project_root: default_true(),
            include_system_info: default_true(),
            include_timestamp: default_true(),
        }
    }
}
impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            output_dir: default_save_dir_config(),
            filename_base: None,
            extension: None,
        }
    }
}
impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            delay: default_watch_delay_string(),
        }
    }
}

impl Config {
    pub fn get_effective_include<'a>(
        &'a self,
        section_include: &'a Option<Vec<String>>,
    ) -> &'a Vec<String> {
        section_include
            .as_ref()
            .unwrap_or(&self.common_filters.include)
    }
    pub fn get_effective_exclude<'a>(
        &'a self,
        section_exclude: &'a Option<Vec<String>>,
    ) -> &'a Vec<String> {
        section_exclude
            .as_ref()
            .unwrap_or(&self.common_filters.exclude)
    }

    pub fn determine_project_root(cli_project_root: Option<&PathBuf>) -> Result<PathBuf> {
        let path_str_opt = cli_project_root
            .map(|p| p.to_string_lossy().to_string())
            .or_else(|| env::var("PROJECT_ROOT").ok().filter(|s| !s.is_empty()));

        let path_to_resolve = match path_str_opt {
            Some(p_str) => PathBuf::from(shellexpand::tilde(&p_str).as_ref()),
            None => env::current_dir().map_err(AppError::Io)?,
        };

        path_to_resolve.canonicalize().map_err(|e| {
            AppError::Io(std::io::Error::new(
                e.kind(),
                format!(
                    "Failed to canonicalize project root '{}': {}",
                    path_to_resolve.display(),
                    e
                ),
            ))
        })
    }

    pub fn resolve_config_path(
        project_root: &Path,
        cli_config_file: Option<&String>,
        cli_disable_config: bool,
    ) -> Result<Option<PathBuf>> {
        if cli_disable_config {
            log::debug!("Config file loading disabled via CLI flag.");
            return Ok(None);
        }

        let path_to_check = match cli_config_file {
            Some(p_str) => {
                let expanded_path_cow = shellexpand::tilde(p_str);
                let mut path = PathBuf::from(expanded_path_cow.as_ref());
                let looks_like_path = path.is_absolute()
                    || path.components().count() > 1
                    || p_str.contains(['/', '\\']);

                if looks_like_path {
                    if !path.exists() && path.extension().is_none() {
                        path.set_extension("toml");
                    }
                    if !path.exists() {
                        return Err(AppError::Config(format!(
                            "Specified config file not found at path: {}",
                            path.display()
                        )));
                    }
                    log::debug!("Using specified config file path: {}", path.display());
                    Some(path)
                } else {
                    let filename = if path.extension().map_or(true, |e| e != "toml") {
                        format!("{}.toml", path.to_string_lossy())
                    } else {
                        path.to_string_lossy().to_string()
                    };
                    let full_path = project_root.join(DEFAULT_CONFIG_DIR).join(filename);
                    if !full_path.exists() {
                        return Err(AppError::Config(format!(
                            "Specified config file '{}' not found in default directory: {}",
                            path.display(),
                            project_root.join(DEFAULT_CONFIG_DIR).display()
                        )));
                    }
                    log::debug!(
                        "Using specified config filename in default directory: {}",
                        full_path.display()
                    );
                    Some(full_path)
                }
            }
            None => {
                let default_path = project_root
                    .join(DEFAULT_CONFIG_DIR)
                    .join(DEFAULT_CONFIG_FILENAME);
                if default_path.exists() {
                    log::debug!("Using default config file path: {}", default_path.display());
                    Some(default_path)
                } else {
                    log::debug!(
                        "No config file specified and default not found at: {}",
                        default_path.display()
                    );
                    None
                }
            }
        };
        Ok(path_to_check)
    }

    pub fn load_from_path(config_path: &Path) -> Result<Self> {
        log::info!("Loading configuration from: {}", config_path.display());
        let toml_content = fs::read_to_string(config_path).map_err(|e| AppError::FileRead {
            path: config_path.to_path_buf(),
            source: e,
        })?;
        toml::from_str::<Config>(&toml_content).map_err(|e| {
            AppError::TomlParse(format!(
                "Error parsing config file '{}': {}. Check TOML syntax and structure.",
                config_path.display(),
                e
            ))
        })
    }

    pub fn get_watch_delay(&self) -> Result<Duration> {
        parse(&self.watch.delay).map_err(|e| {
            AppError::InvalidArgument(format!(
                "Invalid watch delay duration '{}': {}. Use format like '500ms', '2s'.",
                self.watch.delay, e
            ))
        })
    }

    pub fn get_effective_gitignore(&self, section_setting: &IgnoreSetting) -> bool {
        match section_setting {
            IgnoreSetting::True => true,
            IgnoreSetting::False => false,
            IgnoreSetting::Inherit => self.general.use_gitignore,
        }
    }

    pub fn get_effective_builtin_ignore(&self) -> bool {
        self.general.enable_builtin_ignore
    }

    pub fn is_docs_section_active(&self) -> bool {
        self.docs.enabled
    }

    pub fn get_effective_project_name(&self, project_root: &Path) -> String {
        self.general.project_name.clone().unwrap_or_else(|| {
            project_root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "UnknownProject".to_string())
        })
    }
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct ResolvedRules {
    pub rulesets: IndexMap<String, Vec<String>>,
    pub origins: HashMap<String, String>,
}

pub fn resolve_rules(
    rules_config: &RulesConfig,
    project_root: &Path,
    project_characteristics: &HashSet<String>,
) -> Result<ResolvedRules> {
    let mut resolved = ResolvedRules::default();
    if !rules_config.enabled {
        log::debug!("Rules generation is disabled in configuration.");
        return Ok(resolved);
    }
    log::info!("Resolving rules...");

    let mut base_static_stems: HashSet<&str> = rules_mapping::get_default_rule_stems();
    log::trace!("Default rule stems: {:?}", base_static_stems);

    for char in project_characteristics {
        let char_lower = char.to_lowercase();
        if let Some(stem) = rules_mapping::map_characteristic_to_rule_stem(&char_lower)
            .or_else(|| rules_mapping::map_characteristic_to_rule_stem(char))
        {
            if base_static_stems.insert(stem) {
                log::trace!(
                    "Dynamically detected rule stem '{}' from characteristic '{}'",
                    stem,
                    char
                );
            }
        }
    }
    log::debug!(
        "Base static rule stems (defaults + dynamic): {:?}",
        base_static_stems
    );

    let exclude_stems: HashSet<&str> = rules_config.exclude.iter().map(String::as_str).collect();
    if !exclude_stems.is_empty() {
        log::debug!("Applying rule exclusions: {:?}", exclude_stems);
    }

    let mut effective_static_stems: HashSet<&str> = base_static_stems
        .difference(&exclude_stems)
        .copied()
        .collect();
    log::debug!(
        "Static stems after exclusions: {:?}",
        effective_static_stems
    );

    let include_stems: HashSet<&str> = rules_config.include.iter().map(String::as_str).collect();
    if !include_stems.is_empty() {
        log::debug!("Applying explicit rule inclusions: {:?}", include_stems);
    }
    for stem in include_stems.iter() {
        effective_static_stems.insert(stem);
    }
    log::debug!("Final static stems to load: {:?}", effective_static_stems);

    for stem in effective_static_stems.iter() {
        match rules::get_static_rule_content(stem) {
            Ok(content) => {
                let key = format!("static:{}", stem);
                resolved.rulesets.insert(
                    key.clone(),
                    content
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                );
                let origin = match (
                    rules_mapping::get_default_rule_stems().contains(stem),
                    include_stems.contains(stem),
                ) {
                    (true, true) => "default+include",
                    (true, false) => "default",
                    (false, true) => "include",
                    (false, false) => "dynamic",
                };
                resolved.origins.insert(key.clone(), origin.to_string());
                log::trace!("Loaded static rule: {} (Origin: {})", key, origin);
            }
            Err(e) => {
                log::warn!("Skipping static rule stem '{}': {}", stem, e);
            }
        }
    }

    if !rules_config.import.is_empty() {
        log::debug!("Loading imported rules from: {:?}", rules_config.import);
    }
    for import_path_rel in &rules_config.import {
        let mut import_path = project_root.join(import_path_rel);
        if !import_path.exists() {
            let config_dir = project_root.join(DEFAULT_CONFIG_DIR);
            import_path = config_dir.join(import_path_rel);
            if import_path.exists() {
                log::trace!(
                    "Found import {} relative to config dir",
                    import_path_rel.display()
                );
            } else {
                log::warn!(
                    "Could not find imported rule file '{}' relative to project root or config dir. Skipping.",
                    import_path_rel.display()
                );
                continue;
            }
        }

        let stem = import_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported_rule");
        let key = format!("imported:{}", stem);
        match fs::read_to_string(&import_path) {
            Ok(content) => {
                resolved.rulesets.insert(
                    key.clone(),
                    content
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(String::from)
                        .collect(),
                );
                resolved.origins.insert(key.clone(), "import".to_string());
                log::trace!("Loaded imported rule: {}", import_path.display());
            }
            Err(e) => {
                log::warn!(
                    "Failed to read imported rule file '{}': {}",
                    import_path.display(),
                    e
                );
            }
        }
    }

    if !rules_config.custom.is_empty() {
        log::debug!(
            "Loading custom rules defined in config: {:?}",
            rules_config.custom.keys()
        );
    }
    for (name, rules_list) in &rules_config.custom {
        if rules_list.is_empty() {
            log::trace!("Skipping empty custom rule list: {}", name);
            continue;
        }
        let key = format!("custom:{}", name);
        resolved.rulesets.insert(
            key.clone(),
            rules_list
                .iter()
                .map(String::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
        );
        resolved.origins.insert(key.clone(), "custom".to_string());
        log::trace!("Loaded custom rule set: {}", name);
    }
    log::info!("Resolved {} rulesets.", resolved.rulesets.len());
    Ok(resolved)
}

pub fn resolve_prompts(
    prompts_config: &PromptsConfig,
    project_root: &Path,
) -> Result<HashMap<String, String>> {
    let mut resolved = HashMap::new();
    let predefined = crate::output_formats::get_predefined_prompts(); // Assuming get_predefined_prompts moved here
    resolved.extend(
        predefined
            .iter()
            .map(|(k, v)| (format!("static:{}", k), v.clone())),
    );

    if !prompts_config.import.is_empty() {
        log::debug!("Loading imported prompts from: {:?}", prompts_config.import);
    }
    for import_path_rel in &prompts_config.import {
        let mut import_path = project_root.join(import_path_rel);
        if !import_path.exists() {
            let config_dir = project_root.join(DEFAULT_CONFIG_DIR);
            import_path = config_dir.join(import_path_rel);
            if !import_path.exists() {
                log::warn!(
                    "Could not find imported prompt file '{}' relative to project root or config dir. Skipping.",
                    import_path_rel.display()
                );
                continue;
            }
        }

        let stem = import_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported_prompt");
        let key = format!("imported:{}", stem);
        match fs::read_to_string(&import_path) {
            Ok(content) => {
                if !content.trim().is_empty() {
                    resolved.insert(key.clone(), content);
                    log::trace!("Loaded imported prompt: {}", import_path.display());
                } else {
                    log::trace!("Skipping empty imported prompt: {}", import_path.display());
                }
            }
            Err(e) => {
                log::warn!(
                    "Failed to read imported prompt file '{}': {}",
                    import_path.display(),
                    e
                );
            }
        }
    }

    if !prompts_config.custom.is_empty() {
        log::debug!(
            "Loading custom prompts defined in config: {:?}",
            prompts_config.custom.keys()
        );
    }
    for (name, prompt_text) in &prompts_config.custom {
        if !prompt_text.trim().is_empty() {
            let key = format!("custom:{}", name);
            resolved.insert(key.clone(), prompt_text.clone());
            log::trace!("Loaded custom prompt: {}", name);
        } else {
            log::trace!("Skipping empty custom prompt: {}", name);
        }
    }

    log::info!("Resolved {} prompts.", resolved.len());
    Ok(resolved)
}
