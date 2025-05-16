use crate::cli_args::DebugArgs; // Removed unused FormatOutputOpts
use crate::load_config_for_command;
use crate::output::print_data_or_text;
use anyhow::{Context, Result};
use colored::*;
use log;
use pathdiff; // Added use
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use toml; // Added use
use xcontext_core::{self as core, Config, FileInfo, ResolvedRules}; // Removed unused 'config' import alias

#[derive(Debug, Serialize)]
struct DebugInfo<'a> {
    effective_config: &'a Config,
    source_files_to_include: Vec<String>,
    docs_files_to_include: Vec<String>,
    tree_elements_to_include: &'a [(String, bool)], // path, is_dir
    resolved_rules: &'a ResolvedRules,
}

pub fn handle_debug_command(args: DebugArgs, quiet: bool, _verbose: u8) -> Result<()> {
    // Marked verbose as unused
    let project_root = Config::determine_project_root(args.project_config.project_root.as_ref())
        .context("Failed to determine project root")?;
    log::info!("Project root determined: {}", project_root.display());

    let config = load_config_for_command(
        &project_root,
        &args.project_config,
        None,
        None,
        Some(&args.format_output), // Pass format override options
    )
    .context("Failed to load configuration for debug command")?;

    log::debug!("Debug: Gathering file lists...");
    let (source_files, docs_files, tree_path_types) =
        core::gather_files_and_tree(&project_root, &config, quiet)
            .context("Failed to gather file lists for debug")?;
    log::debug!("Debug: File lists gathered.");

    log::debug!("Debug: Detecting project characteristics...");
    let project_characteristics = core::detect_project_characteristics(&project_root)
        .context("Failed to detect project characteristics for debug")?;
    log::debug!("Debug: Characteristics detected.");

    log::debug!("Debug: Resolving rules...");
    let resolved_rules =
        core::config::resolve_rules(&config.rules, &project_root, &project_characteristics)
            .context("Failed to resolve rules for debug")?;
    log::debug!("Debug: Rules resolved.");

    let debug_data = DebugInfo {
        effective_config: &config,
        source_files_to_include: get_relative_paths(&source_files, &project_root),
        docs_files_to_include: get_relative_paths(&docs_files, &project_root),
        tree_elements_to_include: &tree_path_types,
        resolved_rules: &resolved_rules,
    };

    if args.format_output.format.is_none() {
        log::debug!("Debug: Printing pretty output...");
        print_debug_info_pretty(&debug_data, &project_root)?;
        log::debug!("Debug: Pretty output complete.");
    } else {
        log::debug!(
            "Debug: Printing structured output (format: {:?})...",
            args.format_output.format
        );
        // Pass None for plain_text, rely on structured output
        print_data_or_text(&debug_data, None, &args.format_output, "json", "DebugInfo")?;
        log::debug!("Debug: Structured output complete.");
    }
    Ok(())
}

fn get_relative_paths(files: &[FileInfo], project_root: &Path) -> Vec<String> {
    files
        .iter()
        .map(|f| {
            pathdiff::diff_paths(&f.path, project_root) // Added use pathdiff
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| f.path.to_string_lossy().to_string()) // Fallback
        })
        .collect()
}

fn print_debug_info_pretty(debug_info: &DebugInfo, _project_root: &Path) -> Result<()> {
    println!(
        "{}",
        "\n--- Effective Configuration ---"
            .green()
            .bold()
            .underline()
    );
    let config_toml = toml::to_string_pretty(debug_info.effective_config) // Added use toml
        .context("Failed to serialize effective config to TOML")?;
    println!("{}", config_toml);

    print_path_list("Source Files Included", &debug_info.source_files_to_include);
    print_path_list("Docs Files Included", &debug_info.docs_files_to_include);

    println!(
        "{}",
        "\n--- Tree Elements Included ---"
            .green()
            .bold()
            .underline()
    );
    if debug_info.tree_elements_to_include.is_empty() {
        println!("{}", "(None)".dimmed());
    } else {
        // Sort tree elements for consistent output
        let mut sorted_tree = debug_info.tree_elements_to_include.to_vec();
        sorted_tree.sort();
        for (p, is_dir) in sorted_tree {
            let suffix = if is_dir {
                " (dir)".dimmed()
            } else {
                "".normal()
            };
            println!("- {}{}", p.cyan(), suffix);
        }
    }

    display_debug_rules(debug_info.resolved_rules);

    println!("{}", "\n--- End Debug Info ---".green().bold());
    Ok(())
}

fn print_path_list(title: &str, paths: &[String]) {
    println!(
        "{}",
        format!("\n--- {} ---", title).green().bold().underline()
    );
    if paths.is_empty() {
        println!("{}", "(None)".dimmed());
    } else {
        // Assume paths are already sorted from gather step if needed, or sort here
        let mut sorted_paths = paths.to_vec();
        sorted_paths.sort();
        sorted_paths.iter().for_each(|p| println!("- {}", p.cyan()));
    }
}

fn display_debug_rules(resolved_rules: &ResolvedRules) {
    println!("{}", "\n--- Resolved Rules ---".green().bold().underline());
    if resolved_rules.rulesets.is_empty() {
        println!("{}", "(No rules enabled or resolved)".dimmed());
        return;
    }
    println!(
        "{:<35} {:<18} {:<10}",
        "Ruleset Key".bold(),
        "Origin".bold(),
        "Rule Count".bold()
    );
    println!("{:-<65}", ""); // Separator line

    // Sort rules by key for consistent output
    let sorted_rules: BTreeMap<_, _> = resolved_rules.rulesets.iter().collect();

    for (key, rules_list) in sorted_rules {
        let origin_str = resolved_rules
            .origins
            .get(key)
            .map_or("unknown", |s| s.as_str());
        let origin_colored = match origin_str {
            "default" | "default+include" => origin_str.cyan(),
            "dynamic" => origin_str.magenta(),
            "include" => origin_str.green(),
            "import" => origin_str.yellow(),
            "custom" => origin_str.blue(),
            _ => origin_str.dimmed(),
        };
        println!(
            "{:<35} {:<24} {:<10}", // Adjust spacing if needed
            key.blue(),
            origin_colored,
            rules_list.len()
        );
    }
}
