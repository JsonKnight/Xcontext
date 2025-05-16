use crate::cli_args::{FormatOutputOpts, ShowArgs}; // Removed unused Cli, ProjectConfigOpts, ShowItem
use crate::load_config_for_command;
use crate::output::print_data_or_text; // Use CLI output helpers
use anyhow::{Context, Result};
use colored::*;
use log; // Corrected: On its own line
use serde::Serialize; // Needed for ShowOutputWrapper
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use xcontext_core::{self as core, Config, ResolvedRules}; // Removed unused config import alias

#[derive(Serialize)]
struct ShowOutputWrapper<T: Serialize> {
    value: T,
}

pub fn handle_show_command(args: ShowArgs, quiet: bool, verbose: u8) -> Result<()> {
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
    .context("Failed to load configuration for show command")?;

    // Use the full path from cli_args here
    match &args.item {
        crate::cli_args::ShowItem::Meta { key } => {
            handle_show_meta_singular(&config, key.as_deref(), &args.format_output, quiet, verbose)
        }
        crate::cli_args::ShowItem::Metas {} => {
            handle_show_meta_plural(&config, &args.format_output, quiet, verbose)
        }
        crate::cli_args::ShowItem::Prompt { name } => handle_show_prompt_singular(
            &config,
            name.as_deref(),
            &args.format_output,
            quiet,
            verbose,
        ),
        crate::cli_args::ShowItem::Prompts {} => {
            handle_show_prompt_plural(&config, &args.format_output, quiet, verbose)
        }
        crate::cli_args::ShowItem::Rule { name } => handle_show_rule_singular(
            &config,
            name.as_deref(),
            &project_root, // Pass project root for rule resolution
            &args.format_output,
            quiet,
            verbose,
        ),
        crate::cli_args::ShowItem::Rules {} => handle_show_rule_plural(
            &config,
            &project_root, // Pass project root for rule resolution
            &args.format_output,
            quiet,
            verbose,
        ),
    }
}

fn list_available_keys(map: &HashMap<String, String>, item_type: &str, _quiet: bool) {
    // Always print key listing to stderr
    eprintln!("\nAvailable {} keys:", item_type.bold());
    if map.is_empty() {
        eprintln!("  {}", "(None available)".dimmed());
        return;
    }
    let mut sorted_keys: Vec<_> = map.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        eprintln!("  - {}", key.blue());
    }
}

fn list_available_rule_keys(resolved_rules: &ResolvedRules, _quiet: bool) {
    // Always print key listing to stderr
    eprintln!("\nAvailable {} keys:", "Rule Set".yellow().bold());
    if resolved_rules.rulesets.is_empty() {
        eprintln!(
            "  {}",
            "(None available based on current config and project content)".dimmed()
        );
        return;
    }
    let mut sorted_keys: Vec<_> = resolved_rules.rulesets.keys().cloned().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        let origin = resolved_rules.origins.get(&key).map_or("?", |s| s.as_str());
        let origin_colored = match origin {
            "default" | "default+include" => origin.cyan(),
            "dynamic" => origin.magenta(),
            "include" => origin.green(),
            "import" => origin.yellow(),
            "custom" => origin.blue(),
            _ => origin.dimmed(),
        };
        eprintln!("  - {} {}", key.blue(), format!("({})", origin_colored));
    }
}

fn handle_show_meta_singular(
    config: &Config,
    key: Option<&str>,
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if !config.meta.enabled {
        if !quiet {
            eprintln!(
                "{}",
                "Warning: Meta section is disabled in config.".yellow()
            );
        }
        return Ok(());
    }
    let meta_data = &config.meta.custom_meta;
    if let Some(k) = key {
        if let Some(value) = meta_data.get(k) {
            let wrapper = ShowOutputWrapper { value };
            print_data_or_text(
                &wrapper,
                Some(value.clone()),
                format_opts,
                "text",
                "MetaValue",
            )
        } else {
            eprintln!(
                "{} Metadata key \"{}\" not found.",
                "Error:".red(),
                k.blue()
            );
            list_available_keys(meta_data, "metadata", quiet);
            anyhow::bail!("Metadata key not found") // Use anyhow::bail!
        }
    } else {
        if !quiet {
            eprintln!(
                "{}",
                "Please specify a metadata key to show, or use 'show metas' to list all.".yellow()
            );
        }
        list_available_keys(meta_data, "metadata", quiet);
        Ok(())
    }
}

fn handle_show_meta_plural(
    config: &Config,
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if !config.meta.enabled {
        if !quiet {
            eprintln!(
                "{}",
                "Warning: Meta section is disabled in config.".yellow()
            );
        }
        return Ok(());
    }
    let meta_data = &config.meta.custom_meta;
    if meta_data.is_empty() {
        if !quiet {
            println!("No custom metadata defined.");
        }
        return Ok(());
    }

    let sorted_meta: BTreeMap<_, _> = meta_data.iter().collect();

    let pretty_text = if format_opts.format.is_none() {
        let mut output = String::new();
        output.push_str(&format!(
            "{}\n",
            "\n--- Custom Metadata ---".green().bold().underline()
        ));
        for (k, v) in &sorted_meta {
            output.push_str(&format!("  {:<25} : {}\n", k.blue(), v));
        }
        Some(output)
    } else {
        None
    };

    print_data_or_text(&sorted_meta, pretty_text, format_opts, "text", "Metadata")
}

fn handle_show_prompt_singular(
    config: &Config,
    name: Option<&str>,
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let merged_prompts = core::config::resolve_prompts(&config.prompts, Path::new(".")) // Pass dummy path, not needed for static/custom
        .context("Failed to resolve prompts")?;

    if let Some(n) = name {
        let key_to_find = if n.contains(':') {
            n.to_string()
        } else {
            let static_key = format!("static:{}", n);
            let custom_key = format!("custom:{}", n);
            let imported_key = format!("imported:{}", n); // Check imported too
            if merged_prompts.contains_key(&static_key) {
                static_key
            } else if merged_prompts.contains_key(&custom_key) {
                custom_key
            } else if merged_prompts.contains_key(&imported_key) {
                imported_key
            } else {
                n.to_string()
            } // Fallback to original name if prefix missing
        };

        if let Some(text) = merged_prompts.get(&key_to_find) {
            let wrapper = ShowOutputWrapper { value: text };
            print_data_or_text(
                &wrapper,
                Some(text.trim().to_string()),
                format_opts,
                "text",
                "PromptText",
            )
        } else {
            eprintln!("{} Prompt name \"{}\" not found.", "Error:".red(), n.blue());
            list_available_keys(&merged_prompts, "prompt", quiet);
            anyhow::bail!("Prompt name not found")
        }
    } else {
        if !quiet {
            eprintln!(
                "{}",
                "Please specify a prompt name to show, or use 'show prompts' to show all.".yellow()
            );
        }
        list_available_keys(&merged_prompts, "prompt", quiet);
        Ok(())
    }
}

fn handle_show_prompt_plural(
    config: &Config,
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    let merged_prompts = core::config::resolve_prompts(&config.prompts, Path::new(".")) // Pass dummy path
        .context("Failed to resolve prompts")?;

    if merged_prompts.is_empty() {
        if !quiet {
            println!("No prompts available (static, custom, or imported).");
        }
        return Ok(());
    }

    let sorted_prompts: BTreeMap<_, _> = merged_prompts.iter().collect();

    let pretty_text = if format_opts.format.is_none() {
        let mut output = String::new();
        output.push_str(&format!(
            "{}\n",
            "\n--- Available Prompts ---".green().bold().underline()
        ));
        for (key, text) in &sorted_prompts {
            output.push_str(&format!("\n▶ {}:\n", key.blue().bold()));
            for line in text.trim().lines() {
                output.push_str(&format!("    {}\n", line));
            }
        }
        Some(output)
    } else {
        None
    };

    print_data_or_text(&sorted_prompts, pretty_text, format_opts, "text", "Prompts")
}

fn handle_show_rule_singular(
    config: &Config,
    name_with_optional_prefix: Option<&str>,
    project_root: &Path, // Need project root to resolve rules
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if !config.rules.enabled {
        if !quiet {
            eprintln!(
                "{}",
                "Warning: Rules section is disabled in config.".yellow()
            );
        }
        return Ok(());
    }

    let project_characteristics = core::detect_project_characteristics(project_root)
        .context("Failed to detect project characteristics for rule resolution")?;
    let resolved =
        core::config::resolve_rules(&config.rules, project_root, &project_characteristics)
            .context("Failed to resolve rules")?;

    if let Some(name) = name_with_optional_prefix {
        let stem_name = name.split(':').last().unwrap_or(name);
        let potential_keys_to_check = [
            name.to_string(), // Exact match first
            format!("static:{}", stem_name),
            format!("imported:{}", stem_name),
            format!("custom:{}", stem_name),
        ];

        let mut found_key: Option<String> = None;
        let mut found_rules: Option<&Vec<String>> = None;

        for key_candidate in potential_keys_to_check.iter() {
            if let Some(rules_list) = resolved.rulesets.get(key_candidate) {
                found_key = Some(key_candidate.clone());
                found_rules = Some(rules_list);
                break;
            }
        }

        if let (Some(_key), Some(rules_list)) = (found_key, found_rules) {
            let plain_text = rules_list.join("\n");
            let wrapper = ShowOutputWrapper { value: rules_list }; // Wrap the list
            print_data_or_text(&wrapper, Some(plain_text), format_opts, "text", "RuleSet")
        } else {
            eprintln!(
                "{} Rule set name \"{}\" not found in resolved rules.",
                "Error:".red(),
                name.blue()
            );
            list_available_rule_keys(&resolved, quiet);
            anyhow::bail!("Rule set name not found")
        }
    } else {
        if !quiet {
            eprintln!(
                "{}",
                "Please specify a rule set name to show, or use 'show rules' to show all.".yellow()
            );
        }
        list_available_rule_keys(&resolved, quiet);
        Ok(())
    }
}

fn handle_show_rule_plural(
    config: &Config,
    project_root: &Path, // Need project root to resolve rules
    format_opts: &FormatOutputOpts,
    quiet: bool,
    _verbose: u8,
) -> Result<()> {
    if !config.rules.enabled {
        if !quiet {
            eprintln!(
                "{}",
                "Warning: Rules section is disabled in config.".yellow()
            );
        }
        return Ok(());
    }

    let project_characteristics = core::detect_project_characteristics(project_root)
        .context("Failed to detect project characteristics for rule resolution")?;
    let resolved =
        core::config::resolve_rules(&config.rules, project_root, &project_characteristics)
            .context("Failed to resolve rules")?;

    if resolved.rulesets.is_empty() {
        if !quiet {
            println!(
                "No rules available or resolved based on current configuration and project content."
            );
        }
        return Ok(());
    }

    let rules_to_output = &resolved.rulesets;
    let sorted_rules: BTreeMap<_, _> = rules_to_output.iter().collect();

    let pretty_text = if format_opts.format.is_none() {
        let mut output = String::new();
        output.push_str(&format!(
            "{}\n",
            "\n--- Resolved Rule Sets ---".green().bold().underline()
        ));
        for (key, rules) in &sorted_rules {
            let origin = resolved.origins.get(*key).map_or("?", |s| s.as_str());
            let origin_colored = match origin {
                "default" | "default+include" => origin.cyan(),
                "dynamic" => origin.magenta(),
                "include" => origin.green(),
                "import" => origin.yellow(),
                "custom" => origin.blue(),
                _ => origin.dimmed(),
            };
            output.push_str(&format!(
                "\n▶ {} {}:\n",
                key.blue().bold(),
                format!("({})", origin_colored)
            ));
            if rules.is_empty() {
                output.push_str(&format!("  {}\n", "(empty)".dimmed()));
            } else {
                for rule in *rules {
                    output.push_str(&format!("  {}\n", rule));
                }
            }
        }
        Some(output)
    } else {
        None
    };

    print_data_or_text(&sorted_rules, pretty_text, format_opts, "text", "RuleSets")
}
