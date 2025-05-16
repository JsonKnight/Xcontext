mod cli_args;
mod commands;
mod output;
mod watch;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use colored::*;
use log;
use std::process;
// Removed unused Arc import

// Corrected import: Added GenerateArgs
use cli_args::{Cli, Commands, FormatOutputOpts, GenerateArgs, ProjectConfigOpts};
use xcontext_core::{AppError, Config}; // Use Config from core crate

fn main() {
    let cli_args = Cli::parse();

    setup_logging(cli_args.quiet, cli_args.verbose);

    let quiet = cli_args.quiet;
    let verbose = cli_args.verbose;

    log::debug!("CLI args parsed: {:?}", cli_args);

    let exit_code = match run_app(cli_args, quiet, verbose) {
        Ok(_) => {
            log::info!("Application finished successfully.");
            0
        }
        Err(e) => {
            let core_err = e.downcast_ref::<xcontext_core::AppError>();
            let exit_code = match core_err {
                // Define exit codes based on core errors if needed
                Some(AppError::Config(_)) => 1,
                Some(AppError::TomlParse(_)) => 1,
                Some(AppError::TomlSerialize(_)) => 1,
                Some(AppError::Io(_)) => 2,
                Some(AppError::FileRead { .. }) => 2,
                Some(AppError::FileWrite { .. }) => 2,
                Some(AppError::DirCreation { .. }) => 2,
                Some(AppError::WalkDir(_)) => 2,
                Some(AppError::Ignore(_)) => 2,
                Some(AppError::RuleLoading(_)) => 2,
                Some(AppError::Glob(_)) => 2,
                Some(AppError::Chunking(_)) => 3,
                Some(AppError::SystemInfo(_)) => 4,
                Some(AppError::InvalidArgument(_)) => 5,
                // Add mapping for AppError::ClapUsage if you move clap errors to core
                Some(AppError::JsonSerialize(_)) => 6,
                Some(AppError::YamlError(_)) => 6,
                Some(AppError::XmlSerialize(_)) => 6,
                Some(AppError::TikToken(_)) => 8,
                // Add mapping for AppError::WatchError if moved back to core
                Some(AppError::DataLoading(_)) => 1, // Treat data loading like config error
                Some(AppError::DurationParse(_)) => 5, // Treat like invalid arg
                // Corrected: Added wildcard arm for non-exhaustive AppError
                Some(_) => 1, // Default exit code for other *core* AppErrors
                None => 1,    // Default exit code for other *anyhow* errors
            };

            // Only print error if not quiet, or if it's a critical config/clap error
            // This prevents noisy error messages for things handled by core logging
            if !quiet || exit_code == 1 || exit_code == 5 {
                eprintln!("{} {:#}\\n", "Error:".red().bold(), e);
            } else {
                // Log even if quiet for critical errors
                log::error!("Application failed: {:#}", e);
            }

            exit_code
        }
    };
    log::debug!("Exiting with code {}", exit_code);
    process::exit(exit_code);
}

fn setup_logging(quiet: bool, verbose: u8) {
    let log_level = if quiet {
        log::LevelFilter::Off // Turn off logging completely if quiet
    } else {
        match verbose {
            0 => log::LevelFilter::Warn,  // Default: Show warnings and errors
            1 => log::LevelFilter::Info,  // -v: Show info, warnings, errors
            2 => log::LevelFilter::Debug, // -vv: Show debug, info, warnings, errors
            _ => log::LevelFilter::Trace, // -vvv+: Show all levels
        }
    };
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None) // Keep logs clean
        .init();
    log::trace!("Logger initialized with level: {:?}", log_level);
}

fn run_app(cli: Cli, quiet: bool, verbose: u8) -> Result<()> {
    match cli.command {
        None => {
            Cli::command().print_help()?;
        }
        Some(command) => {
            match command {
                Commands::Cl => {
                    log::debug!("Executing 'cl' command...");
                    clearscreen::clear().context("Failed to clear screen")?;
                    log::debug!("Screen cleared.");
                }
                Commands::Completion(args) => {
                    log::debug!("Executing 'completion' command...");
                    commands::completion::handle_completion_command(&args, quiet)?;
                }
                Commands::Config(args) => {
                    log::debug!("Executing 'config' command...");
                    let temp_opts = ProjectConfigOpts::default();
                    let project_root =
                        Config::determine_project_root(temp_opts.project_root.as_ref())
                            .context("Failed to determine project root for config command")?;
                    commands::config::handle_config_command(&args, &project_root, quiet)?;
                }
                Commands::Mcp(_args) => {
                    log::warn!("Executing dummy 'mcp' command...");
                    eprintln!("MCP command not implemented yet.");
                }
                Commands::Generate(args) => {
                    log::debug!("Executing 'generate' command...");
                    commands::generate::handle_generate_command(args, quiet, verbose)?;
                }
                Commands::Watch(args) => {
                    log::debug!("Executing 'watch' command...");
                    // run_watch_mode now takes args directly
                    watch::run_watch_mode(args, quiet, verbose)?;
                }
                Commands::Show(args) => {
                    log::debug!("Executing 'show' command...");
                    commands::show::handle_show_command(args, quiet, verbose)?;
                }
                Commands::Metrics(args) => {
                    log::debug!("Executing 'metrics' command...");
                    commands::metrics::handle_metrics_command(args, quiet)?;
                }
                Commands::Debug(args) => {
                    log::debug!("Executing 'debug' command...");
                    commands::debug::handle_debug_command(args, quiet, verbose)?;
                }
                Commands::Quick(args) => {
                    log::debug!("Executing 'quick' command...");
                    commands::quick::handle_quick_command(args, quiet, verbose)?;
                }
            }
        }
    }
    Ok(())
}

// Kept this function as it seems used by load_config_for_command
fn merge_config_with_cli_overrides(mut config: Config, args: &GenerateArgs) -> Config {
    log::trace!("Applying generate command CLI overrides to config...");

    if let Some(name) = &args.project_config.project_name {
        config.general.project_name = Some(name.clone());
    }

    // Output Format Overrides
    if let Some(format) = &args.format_output.format {
        config.output.format = format.clone();
    }
    // Apply JSON minify logic based on flags and format
    config.output.json_minify = if config.output.format == "json" {
        !args.format_output.disable_json_minify // default to true (minify) unless disable flag is set
    } else {
        true // Default irrelevant for non-JSON, keep consistent default
    };
    // Apply XML pretty print logic based on flags and format
    config.output.xml_pretty_print = if config.output.format == "xml" {
        args.format_output.enable_xml_pretty // default to false (compact) unless enable flag is set
    } else {
        false // Default irrelevant for non-XML
    };

    // Exclusion Overrides
    if args.exclusion.exclude_project_name {
        config.output.include_project_name = false;
    }
    if args.exclusion.exclude_project_root {
        config.output.include_project_root = false;
    }
    if args.exclusion.exclude_timestamp {
        config.output.include_timestamp = false;
    }
    if args.exclusion.exclude_system_info {
        config.output.include_system_info = false;
    }

    // Section Toggle Overrides
    if args.section_toggles.disable_tree {
        config.tree.enabled = false;
    }
    if args.section_toggles.enable_tree {
        config.tree.enabled = true;
    }
    if args.section_toggles.disable_source {
        config.source.enabled = false;
    }
    if args.section_toggles.enable_source {
        config.source.enabled = true;
    }
    if args.section_toggles.disable_meta {
        config.meta.enabled = false;
    }
    if args.section_toggles.enable_meta {
        config.meta.enabled = true;
    }
    if args.section_toggles.disable_rules {
        config.rules.enabled = false;
    }
    if args.section_toggles.enable_rules {
        config.rules.enabled = true;
    }
    if args.section_toggles.disable_docs {
        config.docs.enabled = false;
    }
    if args.section_toggles.enable_docs {
        config.docs.enabled = true;
    }

    // Ignore Toggle Overrides
    if args.ignore_toggles.disable_gitignore {
        config.general.use_gitignore = false;
    }
    if args.ignore_toggles.enable_gitignore {
        config.general.use_gitignore = true;
    }
    if args.ignore_toggles.disable_builtin_ignore {
        config.general.enable_builtin_ignore = false;
    }
    if args.ignore_toggles.enable_builtin_ignore {
        config.general.enable_builtin_ignore = true;
    }

    // Filter Overrides
    if !args.filters.tree_include.is_empty() {
        config.tree.include = Some(args.filters.tree_include.clone());
    }
    if !args.filters.tree_exclude.is_empty() {
        config.tree.exclude = Some(args.filters.tree_exclude.clone());
    }
    if !args.filters.source_include.is_empty() {
        config.source.include = Some(args.filters.source_include.clone());
    }
    if !args.filters.source_exclude.is_empty() {
        config.source.exclude = Some(args.filters.source_exclude.clone());
    }
    if !args.filters.docs_include.is_empty() {
        config.docs.include = Some(args.filters.docs_include.clone());
    }
    if !args.filters.docs_exclude.is_empty() {
        config.docs.exclude = Some(args.filters.docs_exclude.clone());
    }

    // Meta Override
    if !args.meta_override.add_meta.is_empty() {
        log::trace!("Applying meta overrides: {:?}", args.meta_override.add_meta);
        config.meta.enabled = true; // Ensure meta section is enabled if adding via CLI
        for (key, value) in &args.meta_override.add_meta {
            config.meta.custom_meta.insert(key.clone(), value.clone());
        }
    }

    // Watch specific args are handled separately if needed (e.g., in watch command)
    log::trace!("Config after CLI overrides: {:?}", config);
    config
}

// Helper function to load config considering CLI options
// Kept public as it's used by multiple command modules
pub fn load_config_for_command(
    project_root: &std::path::Path,
    project_opts: &ProjectConfigOpts,
    // Pass specific args structs for commands that can override config parts
    generate_args: Option<&cli_args::GenerateArgs>,
    watch_args: Option<&cli_args::WatchArgs>,
    format_override: Option<&FormatOutputOpts>, // For commands like show, metrics, debug, quick
) -> Result<Config> {
    let config_path = Config::resolve_config_path(
        project_root,
        project_opts.context_file.as_ref(),
        project_opts.disable_context_file,
    )
    .context("Failed to resolve configuration path")?;

    let mut config = match &config_path {
        Some(path) => Config::load_from_path(path)
            .with_context(|| format!("Failed to load config from {}", path.display()))?,
        None => Config::default(),
    };

    // Apply overrides from GenerateArgs if provided
    if let Some(gen_args) = generate_args {
        config = merge_config_with_cli_overrides(config, gen_args);
    } else {
        // Apply overrides common to other commands if needed
        if let Some(name) = &project_opts.project_name {
            config.general.project_name = Some(name.clone());
        }
        // Apply format overrides if present
        if let Some(fmt_opts) = format_override {
            if let Some(format) = &fmt_opts.format {
                config.output.format = format.clone();
            }
            config.output.json_minify = if config.output.format == "json" {
                !fmt_opts.disable_json_minify
            } else {
                true
            };
            config.output.xml_pretty_print = if config.output.format == "xml" {
                fmt_opts.enable_xml_pretty
            } else {
                false
            };
        }
        // Apply watch-specific overrides if present
        if let Some(w_args) = watch_args {
            if let Some(delay) = &w_args.watch_delay {
                config.watch.delay = delay.clone();
            }
            // Note: watch also uses format_override logic handled above if needed
        }
    }

    // Ensure project name is set (fallback to directory name)
    config.general.project_name = Some(config.get_effective_project_name(project_root));

    Ok(config)
}
