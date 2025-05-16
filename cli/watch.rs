// Use generate command logic AND the OutputTargetArgs struct from it
use crate::cli_args::WatchArgs;
use crate::commands::generate::{self, OutputTargetArgs};
use crate::load_config_for_command; // Use helper from main
use anyhow::{Context, Result};
use colored::*;
use log;
// CORRECTED: Removed unused NotifyWatcher alias
use notify::{ErrorKind, RecommendedWatcher};
use notify_debouncer_mini::{Debouncer, new_debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use xcontext_core::{self as core, Config}; // Use core types

// Removed Watcher type alias

fn watch_path(
    watcher: &mut Debouncer<RecommendedWatcher>, // Use concrete type
    path: &Path,
    watched_paths: &mut HashSet<PathBuf>,
    quiet: bool,
) -> Result<()> {
    // Attempt to canonicalize, fallback to original path if it fails (e.g., path doesn't exist yet)
    let path_to_watch = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    if !watched_paths.contains(&path_to_watch) && path_to_watch.exists() {
        log::trace!("Attempting to watch: {}", path_to_watch.display());
        match watcher
            .watcher()
            .watch(&path_to_watch, notify::RecursiveMode::NonRecursive) // Watch specific file/dir non-recursively
        {
            Ok(_) => {
                log::debug!("Watching: {}", path_to_watch.display());
                watched_paths.insert(path_to_watch);
            }
            Err(e) => {
                // Don't error out, just log if not quiet
                if !quiet {
                    eprintln!(
                        "{} Failed to watch {}: {}",
                        "⚠️".yellow(),
                        path.display(),
                        e
                    );
                }
                log::warn!("Failed to watch {}: {}", path.display(), e);
            }
        }
    } else if !path_to_watch.exists() {
        log::trace!("Skipping watch for non-existent path: {}", path.display());
    } else {
        log::trace!("Already watching: {}", path_to_watch.display());
    }
    Ok(())
}

fn setup_watches(
    project_root: &Path,
    current_config: &Arc<Config>,
    watcher: &mut Debouncer<RecommendedWatcher>, // Use concrete type
    current_watched: &mut HashSet<PathBuf>,
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    let paths_to_unwatch: Vec<_> = current_watched.iter().cloned().collect();
    log::debug!("Clearing {} previous watches.", paths_to_unwatch.len());
    for path in paths_to_unwatch {
        match watcher.watcher().unwatch(&path) {
            Ok(_) => {
                log::trace!("Unwatched: {}", path.display());
                current_watched.remove(&path);
            }
            Err(e) => match e.kind {
                ErrorKind::WatchNotFound => {
                    log::trace!("Watch not found for {}, removing.", path.display());
                    current_watched.remove(&path); // Remove even if not found
                }
                _ => {
                    if !quiet && verbose > 0 {
                        eprintln!(
                            "{} Failed to unwatch {}: {}",
                            "⚠️".yellow(),
                            path.display(),
                            e
                        );
                    }
                    log::warn!("Failed to unwatch {}: {}", path.display(), e);
                }
            },
        }
    }
    current_watched.clear(); // Ensure it's empty before adding new ones

    log::debug!("Setting up new watches based on current config...");
    match core::gather_files_and_tree(project_root, current_config, quiet) {
        Ok((source_files, docs_files, _)) => {
            if current_config.source.enabled {
                for file_info in source_files {
                    let _ = watch_path(watcher, &file_info.path, current_watched, quiet);
                }
            }
            if current_config.is_docs_section_active() {
                for file_info in docs_files {
                    let _ = watch_path(watcher, &file_info.path, current_watched, quiet);
                }
            }
            // Watch imported rule files
            for rule_import_rel in &current_config.rules.import {
                let mut path = project_root.join(rule_import_rel);
                if !path.exists() {
                    path = project_root
                        .join(core::config::DEFAULT_CONFIG_DIR)
                        .join(rule_import_rel);
                }
                if path.exists() {
                    let _ = watch_path(watcher, &path, current_watched, quiet);
                } else {
                    log::warn!(
                        "Could not find imported rule file to watch: {}",
                        rule_import_rel.display()
                    );
                }
            }
            // Watch imported prompt files
            for prompt_import_rel in &current_config.prompts.import {
                let mut path = project_root.join(prompt_import_rel);
                if !path.exists() {
                    path = project_root
                        .join(core::config::DEFAULT_CONFIG_DIR)
                        .join(prompt_import_rel);
                }
                if path.exists() {
                    let _ = watch_path(watcher, &path, current_watched, quiet);
                } else {
                    log::warn!(
                        "Could not find imported prompt file to watch: {}",
                        prompt_import_rel.display()
                    );
                }
            }
        }
        Err(e) => {
            if !quiet {
                eprintln!(
                    "{} {}",
                    "⚠️ Error gathering files for watch setup:".yellow(),
                    e
                );
            }
        }
    }

    let config_path_to_watch = Config::resolve_config_path(
        project_root,
        None,  // Rely on default resolution logic here for watching
        false, // Don't disable checking for the config file
    )?;
    if let Some(ref config_path) = config_path_to_watch {
        let _ = watch_path(watcher, config_path, current_watched, quiet);
    }

    if current_watched.is_empty() {
        if !quiet {
            println!(
                "{}",
                "⚠️ No source, documentation, import, or config files found to watch based on current configuration."
                    .yellow()
            );
        }
    } else if !quiet && verbose > 0 {
        println!("🔍 Watching {} files/paths...", current_watched.len());
    }
    Ok(())
}

pub fn run_watch_mode(watch_args: WatchArgs, quiet: bool, verbose: u8) -> Result<()> {
    let project_root =
        Config::determine_project_root(watch_args.project_config.project_root.as_ref())
            .context("Failed to determine project root for watch mode")?;

    if !quiet {
        println!(
            "👀 Starting watch mode for '{}'. Press Ctrl+C to exit.",
            project_root.display()
        );
    }

    let mut config = Arc::new(
        load_config_for_command(
            &project_root,
            &watch_args.project_config,
            None, // No generate args
            Some(&watch_args),
            None, // Format handled by generate call
        )
        .context("Failed to load initial configuration for watch mode")?,
    );

    let initial_output_target_args = OutputTargetArgs {
        save: &watch_args.save,
        chunks: &None, // Watch mode doesn't support chunking trigger
        stdout: watch_args.save.is_none(), // Default to stdout if not saving
        format_output: &watch_args.format_output,
    };

    if let Err(e) = generate::trigger_generation(
        &project_root,
        &config,
        &initial_output_target_args,
        quiet,
        verbose,
    ) {
        if !quiet {
            eprintln!("{} {}\n", "⚠️ Error during initial generation:".yellow(), e);
        }
    } else if !quiet && verbose > 0 {
        println!("{}\n", "✅ Initial generation complete.".green());
    }

    let (tx, rx) = mpsc::channel();
    let delay_duration = config
        .get_watch_delay()
        .with_context(|| "Invalid watch delay duration")?;
    let mut debouncer = new_debouncer(delay_duration, tx)
        .map_err(|e| anyhow::anyhow!("Failed to create debouncer: {}", e))?;
    let mut watched_paths = HashSet::new();

    if let Err(e) = setup_watches(
        &project_root,
        &config,
        &mut debouncer,
        &mut watched_paths,
        quiet,
        verbose,
    ) {
        if !quiet {
            eprintln!(
                "{} {}\n",
                "⚠️ Error setting up initial watches:".yellow(),
                e
            );
        }
    }

    loop {
        match rx.recv() {
            Ok(event_result) => match event_result {
                Ok(debounced_events) => {
                    if !debounced_events.is_empty() {
                        if !quiet && verbose > 0 {
                            eprintln!(
                                "\n{} {} event(s) detected.",
                                "🔄".blue(),
                                debounced_events.len()
                            );
                            for event in &debounced_events {
                                log::trace!("Debounced event: {:?}", event);
                            }
                        }

                        let config_path_being_used_result = Config::resolve_config_path(
                            &project_root,
                            watch_args.project_config.context_file.as_ref(),
                            watch_args.project_config.disable_context_file,
                        );

                        let config_changed = if let Ok(Some(ref current_config_path)) =
                            config_path_being_used_result
                        {
                            let canonical_config_path = current_config_path.canonicalize().ok();
                            debounced_events.iter().any(|event| {
                                let event_path_canonical = event.path.canonicalize().ok();
                                match (&canonical_config_path, &event_path_canonical) {
                                    (Some(conf_canon), Some(evt_canon)) => conf_canon == evt_canon,
                                    _ => event.path == *current_config_path,
                                }
                            })
                        } else {
                            false
                        };

                        let mut config_reloaded = false;
                        if config_changed {
                            if !quiet && verbose > 0 {
                                eprintln!(
                                    "{}",
                                    "🔄 Config file changed. Reloading configuration...".blue()
                                );
                            }
                            match load_config_for_command(
                                &project_root,
                                &watch_args.project_config,
                                None,
                                Some(&watch_args),
                                None,
                            ) {
                                Ok(reloaded_config) => {
                                    config = Arc::new(reloaded_config);
                                    config_reloaded = true;
                                    if !quiet && verbose > 0 {
                                        eprintln!("{}", "✅ Configuration reloaded.".green());
                                    }
                                    if let Err(e) = setup_watches(
                                        &project_root,
                                        &config,
                                        &mut debouncer,
                                        &mut watched_paths,
                                        quiet,
                                        verbose,
                                    ) {
                                        if !quiet {
                                            eprintln!(
                                                "{} {}",
                                                "⚠️ Error setting up watches after config reload:"
                                                    .yellow(),
                                                e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    if !quiet {
                                        eprintln!(
                                            "{} {:#}\n",
                                            "⚠️ Error reloading config:".yellow(),
                                            e
                                        );
                                    }
                                }
                            }
                        }

                        if !quiet && verbose > 0 && !config_reloaded {
                            eprintln!("{}", "\n🔄 Regenerating context...".blue());
                        } else if !quiet && verbose > 0 && config_reloaded {
                            // Already printed message
                        }

                        let output_target_args = OutputTargetArgs {
                            save: &watch_args.save,
                            chunks: &None,
                            stdout: watch_args.save.is_none(),
                            format_output: &watch_args.format_output,
                        };

                        if let Err(e) = generate::trigger_generation(
                            &project_root,
                            &config,
                            &output_target_args,
                            quiet,
                            verbose,
                        ) {
                            if !quiet {
                                eprintln!("{} {:#}\n", "⚠️ Error during regeneration:".yellow(), e);
                            }
                        } else if !quiet && verbose > 0 {
                            println!("{}\n", "✅ Regeneration complete.".green());
                        }

                        if !quiet && verbose > 0 && !watched_paths.is_empty() {
                            println!("🔍 Watching {} files/paths...", watched_paths.len());
                        } else if !quiet && verbose > 0 && watched_paths.is_empty() {
                            println!("🔍 No files currently being watched.");
                        }
                    } else {
                        log::trace!("Received empty debounced event list.");
                    }
                }
                Err(error) => {
                    if !quiet {
                        eprintln!("{} {:#}\n", "⚠️ Watch error:".yellow(), error);
                    }
                    log::error!("Notify error received: {:?}", error);
                }
            },
            Err(e) => {
                eprintln!("{} {:#}\n", "⛔ Watcher channel error:".red(), e);
                break Ok(());
            }
        }
    }
}
