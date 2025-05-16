use crate::cli_args::QuickArgs; // Removed unused FormatOutputOpts
use crate::load_config_for_command;
use crate::output::print_data_or_text;
use anyhow::{Context, Result};
use colored::*;
use glob::Pattern;
use ignore::{WalkBuilder, WalkState};
use log;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
// Removed: use rayon::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf; // Removed unused Path import
use std::sync::mpsc;
use xcontext_core::Config;

#[derive(Debug, Serialize)]
struct QuickOutput {
    files: HashMap<String, String>,
}

pub fn handle_quick_command(args: QuickArgs, quiet: bool, verbose: u8) -> Result<()> {
    let project_root = Config::determine_project_root(args.project_config.project_root.as_ref())
        .context("Failed to determine project root")?;
    log::info!("Project root determined: {}", project_root.display());

    // Load config primarily to respect ignore rules (.gitignore, built-in)
    let config = load_config_for_command(
        &project_root,
        &args.project_config,
        None,
        None,
        Some(&args.format_output), // Pass format override options
    )
    .context("Failed to load configuration for quick command")?;

    let mut pattern_to_use = args.pattern.clone();
    let potential_path = project_root.join(&args.pattern);
    let mut info_msg = None;

    // Check if the pattern looks like a directory and adjust glob
    if potential_path.is_dir() {
        pattern_to_use = format!(
            "{}**/*",
            args.pattern.trim_end_matches(&['/', '\\'] as &[char])
        );
        info_msg = Some(format!(
            "{} Interpreting directory input '{}' as glob '{}'",
            "ℹ️".blue(),
            args.pattern,
            pattern_to_use
        ));
    } else if args.pattern.ends_with(&['/', '\\'] as &[char]) {
        // If it ends with slash but isn't a dir, warn and use modified pattern
        if !quiet {
            eprintln!(
                "{} Directory pattern '{}' matches no existing directory, using pattern without trailing slash.",
                "⚠️".yellow(),
                args.pattern
            );
        }
        pattern_to_use = args
            .pattern
            .trim_end_matches(&['/', '\\'] as &[char])
            .to_string();
    }

    if let Some(msg) = info_msg {
        if !quiet && verbose > 0 {
            eprintln!("{}", msg);
        }
    }

    let glob_pattern = Pattern::new(&pattern_to_use).with_context(|| {
        format!(
            "Invalid glob pattern for quick: '{}' (processed as '{}')",
            args.pattern, pattern_to_use
        )
    })?;

    let use_gitignore = config.general.use_gitignore;
    let _enable_builtin_ignore = config.general.enable_builtin_ignore; // TODO: Apply built-in ignores too?

    let mut builder = WalkBuilder::new(&project_root);
    builder.hidden(false); // Include hidden files unless filtered by ignores
    builder.ignore(use_gitignore);
    builder.git_ignore(use_gitignore);
    builder.git_exclude(use_gitignore);
    builder.require_git(false);
    // TODO: Add logic to apply built-in ignores here if desired for `quick`

    let walker = builder.build_parallel();
    let (tx_path, rx_path) = mpsc::channel::<PathBuf>();
    let glob_pattern_outer_clone = glob_pattern.clone(); // Clone for closure
    let proj_root_clone = project_root.clone(); // Clone for closure

    log::debug!(
        "Starting parallel walk for pattern: {}",
        glob_pattern.as_str()
    );
    walker.run(move || {
        // tx_path is MOVED here
        let tx = tx_path.clone(); // Clone the moved sender for the inner closure
        let proj_root_inner = proj_root_clone.clone();
        let glob_pattern_inner_clone = glob_pattern_outer_clone.clone();

        Box::new(move |entry_result| {
            if let Ok(entry) = entry_result {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    if let Some(relative_path) =
                        pathdiff::diff_paths(entry.path(), &proj_root_inner)
                    {
                        if glob_pattern_inner_clone.matches_path(&relative_path) {
                            log::trace!("Matched file: {}", relative_path.display());
                            // Send using the cloned sender for this thread
                            let _ = tx.send(entry.path().to_path_buf());
                        }
                    } else if glob_pattern_inner_clone.matches_path(entry.path()) {
                        log::trace!("Matched absolute path: {}", entry.path().display());
                        let _ = tx.send(entry.path().to_path_buf());
                    }
                }
            }
            WalkState::Continue
        })
    }); // The original tx_path (owned by the closure) goes out of scope here

    // Removed: drop(tx_path); // No longer needed, and tx_path was moved

    let paths_to_read: Vec<_> = rx_path.into_iter().collect(); // This will finish when all senders (clones) are dropped
    log::info!(
        "Found {} files matching pattern. Reading content...",
        paths_to_read.len()
    );

    let results: Vec<Result<(String, String)>> = paths_to_read
        .par_iter()
        .map(|path| {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read file {}", path.display()))?;
            let relative_path = pathdiff::diff_paths(path, &project_root)
                .unwrap_or_else(|| path.clone())
                .to_string_lossy()
                .to_string();
            Ok((relative_path, content))
        })
        .collect();

    let mut files_map = HashMap::new();
    let mut read_errors = Vec::new();

    for result in results {
        match result {
            Ok((path_str, content)) => {
                files_map.insert(path_str, content);
            }
            Err(e) => {
                read_errors.push(format!("{:#}", e));
            }
        }
    }

    if !read_errors.is_empty() && !quiet {
        eprintln!(
            "{}",
            "Warning: Errors encountered during file reading:".yellow()
        );
        for err_msg in read_errors {
            eprintln!(" - {}", err_msg);
        }
        eprintln!("---");
    }

    if files_map.is_empty() && !quiet {
        println!("No files matched the pattern '{}'.", args.pattern);
        return Ok(());
    }

    let output_data = QuickOutput { files: files_map };

    let default_format = "json";
    print_data_or_text(
        &output_data,
        None,
        &args.format_output,
        default_format,
        "QuickOutput",
    )
}
