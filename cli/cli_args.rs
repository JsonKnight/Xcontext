use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Args, Debug, Clone, Default)]
pub struct ProjectConfigOpts {
    #[arg(
        long,
        help = "Specify the target project directory (default: current dir).",
        help_heading = "Project Setup",
        value_name = "PATH"
    )]
    pub project_root: Option<PathBuf>,

    #[arg(
        long,
        help = "Specify path/filename of the TOML config file (default: .xtools/xcontext/xcontext.toml).",
        value_name = "CONTEXT_FILE",
        conflicts_with = "disable_context_file",
        help_heading = "Project Setup"
    )]
    pub context_file: Option<String>,

    #[arg(
        long,
        help = "Disable loading any TOML config file.",
        conflicts_with = "context_file",
        help_heading = "Project Setup"
    )]
    pub disable_context_file: bool,

    #[arg(
        long,
        help = "Specify the project name (overrides config/dir name).",
        value_name = "NAME",
        help_heading = "Project Setup"
    )]
    pub project_name: Option<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct FormatOutputOpts {
    #[arg(short = 'f', long, help = "Set the output format.", value_name = "FORMAT", value_parser = ["json", "yaml", "xml"], help_heading = "Output Formatting")]
    pub format: Option<String>,

    #[arg(
        long,
        help = "Ensure JSON output is compact (minified) [default].",
        conflicts_with = "disable_json_minify",
        help_heading = "Output Formatting"
    )]
    pub enable_json_minify: bool,

    #[arg(
        long,
        help = "Ensure JSON output is pretty-printed (readable).",
        conflicts_with = "enable_json_minify",
        help_heading = "Output Formatting"
    )]
    pub disable_json_minify: bool,

    #[arg(
        long,
        help = "Ensure XML output is pretty-printed (readable).",
        conflicts_with = "disable_xml_pretty",
        help_heading = "Output Formatting"
    )]
    pub enable_xml_pretty: bool,

    #[arg(
        long,
        help = "Ensure XML output is compact [default].",
        conflicts_with = "enable_xml_pretty",
        help_heading = "Output Formatting"
    )]
    pub disable_xml_pretty: bool,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Generate structured project context for AI models.",
    long_about = "xcontext scans project files based on configuration and generates context \n(metadata, system info, structure, code, docs, rules) suitable for AI processing. \nSupports multiple output formats, filtering, and utility modes.",
    help_template = "{about-section}\nUsage: {usage}\n\n{all-args}{after-help}",
    after_help = "EXAMPLES:\n  xcontext generate -f yaml --save ./output\n  xcontext show rules -f json\n  xcontext metrics\n  xcontext watch -s",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Increase message verbosity (-v, -vv).")]
    pub verbose: u8,

    #[arg(
        short,
        long,
        global = true,
        help = "Silence informational messages and warnings."
    )]
    pub quiet: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    #[command(
        visible_alias = "g",
        visible_alias = "gen",
        about = "Generate the full project context."
    )]
    Generate(GenerateArgs),

    #[command(
        visible_alias = "w",
        about = "Monitor project files and regenerate context automatically."
    )]
    Watch(WatchArgs),

    #[command(
        visible_alias = "s",
        about = "Show specific configured items (metadata, prompts, rules)."
    )]
    Show(ShowArgs),

    #[command(
        visible_alias = "m",
        about = "Calculate and display project statistics."
    )]
    Metrics(MetricsArgs),

    #[command(
        visible_alias = "d",
        about = "Show effective configuration and planned file inclusions."
    )]
    Debug(DebugArgs),

    #[command(
        visible_alias = "q",
        about = "Quickly extract content of files matching a pattern."
    )]
    Quick(QuickArgs),

    #[command(about = "Generate or save shell completion scripts.")]
    Completion(CompletionArgs),

    #[command(about = "Show or save the default configuration file structure.")]
    Config(ConfigArgs),

    #[command(visible_alias = "c", about = "Clear the terminal screen.")]
    Cl,

    #[command(about = "Dummy MCP command (placeholder).")]
    Mcp(McpArgs),
}

#[derive(Args, Debug, Clone)]
pub struct GenerateArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,

    #[arg(
        long,
        help = "Force output of the main context to standard output.",
        help_heading = "Output Control",
        conflicts_with = "save"
    )]
    pub stdout: bool,

    #[arg(
        short = 's', long, value_name = "SAVE_DIR",
        num_args = 0..=1,
        help_heading = "Output Control",
        help = "Save context. Optional SAVE_DIR overrides config/default logic.",
    )]
    pub save: Option<Option<PathBuf>>,

    #[arg(
        short = 'c',
        long,
        help = "Split source content into chunks (e.g., '5MB', '1024kb'). Requires JSON format.",
        value_name = "SIZE_STRING",
        help_heading = "Output Control"
    )]
    pub chunks: Option<String>,

    #[clap(flatten)]
    pub exclusion: ExclusionGroup,
    #[clap(flatten)]
    pub section_toggles: SectionTogglesGroup,
    #[clap(flatten)]
    pub ignore_toggles: IgnoreTogglesGroup,
    #[clap(flatten)]
    pub filters: FilterGroup,
    #[clap(flatten)]
    pub meta_override: MetaOverrideGroup,
}

#[derive(Args, Debug, Clone)]
pub struct WatchArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,

    #[arg(
        long,
        value_name = "DELAY_STRING",
        help = "Set debounce delay for watch mode [default: 300ms]"
    )]
    pub watch_delay: Option<String>,

    #[arg( short = 's', long, value_name = "SAVE_DIR", num_args = 0..=1, help = "Save context on change. Optional SAVE_DIR overrides config/default logic.", )]
    pub save: Option<Option<PathBuf>>,
}

#[derive(Args, Debug, Clone)]
pub struct ShowArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,
    #[command(subcommand)]
    pub item: ShowItem,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ShowItem {
    #[command(about = "Show specific metadata key or list available keys.")]
    Meta { key: Option<String> },
    #[command(about = "Show content of all metadata keys (default: pretty text).")]
    Metas {},
    #[command(about = "Show specific prompt or list available prompt names.")]
    Prompt { name: Option<String> },
    #[command(about = "Show content of all prompts (default: pretty text).")]
    Prompts {},
    #[command(about = "Show specific rule set/list or list available names.")]
    Rule { name: Option<String> },
    #[command(about = "Show content of all rule sets/lists (default: pretty text).")]
    Rules {},
}

#[derive(Args, Debug, Clone)]
pub struct MetricsArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,
}

#[derive(Args, Debug, Clone)]
pub struct DebugArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,
}

#[derive(Args, Debug, Clone)]
pub struct QuickArgs {
    #[clap(flatten)]
    pub project_config: ProjectConfigOpts,
    #[clap(flatten)]
    pub format_output: FormatOutputOpts,
    #[arg(
        required = true,
        help = "Glob pattern (e.g., 'src/**/*.rs', 'data/', 'file.txt')"
    )]
    pub pattern: String,
}

#[derive(Args, Debug, Clone)]
pub struct CompletionArgs {
    #[arg(
        long,
        value_name = "SHELL",
        help = "Shell to generate completions for (fish, bash, zsh) [default: fish]"
    )]
    pub shell: Option<String>,
    #[arg(
        long,
        help = "Save completion script to default location (prompts overwrite)."
    )]
    pub save: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    #[arg(
        long,
        help = "Save default config structure to default path (prompts overwrite)."
    )]
    pub save: bool,
}

#[derive(Args, Debug, Clone)]
pub struct McpArgs {}

#[derive(Args, Debug, Clone, Default)]
pub struct ExclusionGroup {
    #[arg(
        long,
        help = "Omit 'project_name' field from output.",
        help_heading = "Core Exclusions"
    )]
    pub exclude_project_name: bool,
    #[arg(
        long,
        help = "Omit 'project_root' field from output.",
        help_heading = "Core Exclusions"
    )]
    pub exclude_project_root: bool,
    #[arg(
        long,
        help = "Omit 'generation_timestamp' field from output.",
        help_heading = "Core Exclusions"
    )]
    pub exclude_timestamp: bool,
    #[arg(
        long,
        help = "Omit 'system_info' field from output.",
        help_heading = "Core Exclusions"
    )]
    pub exclude_system_info: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct SectionTogglesGroup {
    #[arg(
        long,
        help = "Force inclusion of the 'tree' section [default: enabled].",
        overrides_with = "disable_tree",
        help_heading = "Section Toggles"
    )]
    pub enable_tree: bool,
    #[arg(
        long,
        help = "Disable the 'tree' section.",
        overrides_with = "enable_tree",
        help_heading = "Section Toggles"
    )]
    pub disable_tree: bool,

    #[arg(
        long,
        help = "Force inclusion of the 'source' section [default: enabled].",
        overrides_with = "disable_source",
        help_heading = "Section Toggles"
    )]
    pub enable_source: bool,
    #[arg(
        long,
        help = "Disable the 'source' section.",
        overrides_with = "enable_source",
        help_heading = "Section Toggles"
    )]
    pub disable_source: bool,

    #[arg(
        long,
        help = "Force inclusion of the 'meta' section [default: enabled].",
        overrides_with = "disable_meta",
        help_heading = "Section Toggles"
    )]
    pub enable_meta: bool,
    #[arg(
        long,
        help = "Disable the 'meta' section.",
        overrides_with = "enable_meta",
        help_heading = "Section Toggles"
    )]
    pub disable_meta: bool,

    #[arg(
        long,
        help = "Force inclusion of the 'rules' section [default: enabled].",
        overrides_with = "disable_rules",
        help_heading = "Section Toggles"
    )]
    pub enable_rules: bool,
    #[arg(
        long,
        help = "Disable the 'rules' section.",
        overrides_with = "enable_rules",
        help_heading = "Section Toggles"
    )]
    pub disable_rules: bool,

    #[arg(
        long,
        help = "Force inclusion of the 'docs' section [default: enabled].",
        overrides_with = "disable_docs",
        help_heading = "Section Toggles"
    )]
    pub enable_docs: bool,
    #[arg(
        long,
        help = "Disable the 'docs' section.",
        overrides_with = "enable_docs",
        help_heading = "Section Toggles"
    )]
    pub disable_docs: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct IgnoreTogglesGroup {
    #[arg(
        long,
        help = "Globally enable respecting .gitignore files [default: enabled].",
        overrides_with = "disable_gitignore",
        help_heading = "Ignore Rules"
    )]
    pub enable_gitignore: bool,
    #[arg(
        long,
        help = "Globally disable respecting .gitignore files.",
        overrides_with = "enable_gitignore",
        help_heading = "Ignore Rules"
    )]
    pub disable_gitignore: bool,

    #[arg(
        long,
        help = "Enable default built-in ignores (e.g., *.lock, target/) [default: enabled].",
        overrides_with = "disable_builtin_ignore",
        help_heading = "Ignore Rules"
    )]
    pub enable_builtin_ignore: bool,
    #[arg(
        long,
        help = "Disable default built-in ignores.",
        overrides_with = "enable_builtin_ignore",
        help_heading = "Ignore Rules"
    )]
    pub disable_builtin_ignore: bool,
}

#[derive(Args, Debug, Clone, Default)]
pub struct FilterGroup {
    #[arg(long = "tree-include", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add include path/glob pattern for tree view.", help_heading = "Content Filtering")]
    pub tree_include: Vec<String>,
    #[arg(long = "tree-exclude", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add exclude path/glob pattern for tree view.", help_heading = "Content Filtering")]
    pub tree_exclude: Vec<String>,

    #[arg(long = "source-include", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add include path/glob pattern for source files.", help_heading = "Content Filtering")]
    pub source_include: Vec<String>,
    #[arg(long = "source-exclude", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add exclude path/glob pattern for source files.", help_heading = "Content Filtering")]
    pub source_exclude: Vec<String>,

    #[arg(long = "docs-include", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add include path/glob pattern for documentation files.", help_heading = "Content Filtering")]
    pub docs_include: Vec<String>,
    #[arg(long = "docs-exclude", value_name = "PATTERN", action = clap::ArgAction::Append, help = "Add exclude path/glob pattern for documentation files.", help_heading = "Content Filtering")]
    pub docs_exclude: Vec<String>,
}

#[derive(Args, Debug, Clone, Default)]
pub struct MetaOverrideGroup {
    #[arg(long = "add-meta", value_name = "key=value", value_parser = parse_key_val, action = clap::ArgAction::Append, help = "Add/override key=value pairs in the 'meta' section.", help_heading = "Metadata Override")]
    pub add_meta: Vec<(String, String)>,
}

fn parse_key_val(s: &str) -> std::result::Result<(String, String), String> {
    s.find('=')
        .map(|idx| {
            let key = s[..idx].trim().to_string();
            let value = s[idx + 1..].trim().to_string();
            if key.is_empty() {
                Err("Metadata key cannot be empty".to_string())
            } else {
                Ok((key, value))
            }
        })
        .ok_or_else(|| "Invalid KEY=VALUE format for --add-meta".to_string())?
}
