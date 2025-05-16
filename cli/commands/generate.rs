use crate::cli_args::GenerateArgs; // Removed unused WatchArgs
use crate::load_config_for_command;
use crate::output;
use anyhow::{Context, Result};
use colored::Colorize;
use log;
use std::fs; // Added use std::fs
use std::path::{Path, PathBuf};
use std::sync::Arc;
use xcontext_core::{self as core, Config, ProjectContext}; // Use core types

pub fn handle_generate_command(args: GenerateArgs, quiet: bool, verbose: u8) -> Result<()> {
    let project_root = Config::determine_project_root(args.project_config.project_root.as_ref())
        .context("Failed to determine project root")?;
    log::info!("Project root determined: {}", project_root.display());

    let config = Arc::new(
        load_config_for_command(
            &project_root,
            &args.project_config,
            Some(&args), // Pass generate args for overrides
            None,
            None, // Format handled within load_config_for_command via generate_args
        )
        .context("Failed to load configuration")?,
    );

    // Create OutputTargetArgs from GenerateArgs
    let output_target_args = OutputTargetArgs {
        save: &args.save,
        chunks: &args.chunks,
        stdout: args.stdout,
        format_output: &args.format_output,
    };

    // Use trigger_generation which handles the core logic + output
    trigger_generation(&project_root, &config, &output_target_args, quiet, verbose) // Pass correct type
}

// This function now encapsulates the core generation logic
// It's called by both `generate` and `watch` commands
// Made public so watch.rs can use it
pub fn trigger_generation(
    project_root: &Path,
    config: &Arc<Config>,
    output_target_args: &OutputTargetArgs, // Now expects this type
    quiet: bool,
    verbose: u8,
) -> Result<()> {
    log::info!(
        "Starting context generation for: {}",
        project_root.display()
    );

    validate_args_for_generation(config, output_target_args)?;

    log::debug!("Gathering files and tree elements...");
    let (source_files, docs_files, tree_path_types) =
        core::gather_files_and_tree(project_root, config, quiet)
            .context("Failed to gather project files and directory structure")?;
    log::debug!(
        "Gathering complete. Found {} source, {} docs, {} tree elements.",
        source_files.len(),
        docs_files.len(),
        tree_path_types.len()
    );

    let tree_for_context: Option<Vec<core::TreeNode>> = if config.tree.enabled {
        log::debug!("Building tree structure...");
        let tree = core::gather::build_tree_from_paths(&tree_path_types)
            .context("Failed to build directory tree structure")?;
        log::debug!("Tree structure built.");
        Some(tree)
    } else {
        log::debug!("Tree structure disabled in config.");
        None
    };

    log::debug!("Detecting project characteristics...");
    let project_characteristics = core::detect_project_characteristics(project_root)
        .context("Failed to detect project characteristics")?;
    log::debug!("Characteristics detected: {:?}", project_characteristics);

    log::debug!("Building initial project context (including rule resolution)...");
    let mut main_context = ProjectContext::build(
        project_root,
        config,
        tree_for_context,
        &project_characteristics,
    )
    .context("Failed to build initial project context")?;
    log::debug!("Initial context built.");

    // Add docs if enabled
    main_context = main_context.add_docs(docs_files, project_root, config);

    // Handle source files (inline or chunking)
    if config.source.enabled {
        log::debug!("Processing source files...");
        if let Some(chunk_size_str) = output_target_args.chunks.as_deref() {
            log::info!("Chunking source files with size: {}", chunk_size_str);

            let (save_dir, filename_base, _) =
                get_save_details_from_args(config, output_target_args.save.as_ref(), project_root);

            let chunk_files_data =
                core::chunking::split_files_into_chunks(source_files, chunk_size_str, project_root)
                    .context("Failed to split files into chunks")?;

            let mut chunk_file_paths = Vec::<PathBuf>::new();
            if !chunk_files_data.is_empty() {
                fs::create_dir_all(&save_dir).with_context(|| {
                    // Added std::fs import
                    format!(
                        "Failed to create chunk output directory {}",
                        save_dir.display()
                    )
                })?;
            }

            for (i, chunk_data) in chunk_files_data.iter().enumerate() {
                let chunk_num = i + 1;
                let chunk_filename = format!("{}_chunk_{}.json", filename_base, chunk_num); // Chunks always JSON
                let chunk_path = save_dir.join(&chunk_filename);
                // Use output::save_chunk_file, passing format_opts from output_target_args
                output::save_chunk_file(
                    chunk_data,
                    &chunk_path,
                    &output_target_args.format_output,
                    quiet,
                )?;
                chunk_file_paths.push(chunk_path);
            }

            main_context = main_context.add_chunk_paths(chunk_file_paths, &save_dir, config);
            log::info!("Chunking processing complete.");

            // Output the main context file (without sources, just chunk refs) if saving is requested
            if output_target_args.save.is_some() {
                let (main_save_dir, main_filename_base, main_extension) =
                    get_save_details_from_args(
                        config,
                        output_target_args.save.as_ref(),
                        project_root,
                    );
                let main_filename = format!("{}.{}", main_filename_base, main_extension);
                let main_output_path = main_save_dir.join(main_filename);
                log::info!(
                    "Saving main context (with chunk references) to file: {}",
                    main_output_path.display()
                );
                output::print_context_or_save(
                    &main_context,
                    config,
                    Some(&main_output_path),
                    &output_target_args.format_output,
                    quiet,
                )?;
            } else if output_target_args.stdout {
                // If stdout is forced even with chunking, print the main context (with chunk refs)
                log::info!("Outputting main context (with chunk references) to stdout...");
                output::print_context_or_save(
                    &main_context,
                    config,
                    None,
                    &output_target_args.format_output,
                    quiet,
                )?;
            } else if !quiet {
                // Print confirmation about chunks being saved if not outputting main context elsewhere
                println!(
                    "{} Source content chunked and saved in: {}",
                    "✅".green(),                          // Added Colorize import
                    save_dir.display().to_string().blue()  // Added Colorize import
                );
            }
        } else {
            log::debug!("Adding source files inline...");
            main_context = main_context.add_files(source_files, project_root, config);
            // Output main context (with inline sources)
            handle_final_output(
                &main_context,
                config,
                output_target_args,
                project_root,
                quiet,
            )?;
        }
    } else {
        log::debug!("Source section disabled.");
        if !source_files.is_empty() && !quiet && verbose > 0 {
            eprintln!(
                "{}",
                "Warning: Source section disabled, but source files were found and ignored."
                    .yellow() // Added Colorize import
            );
        }
        // Output main context (without any source section)
        handle_final_output(
            &main_context,
            config,
            output_target_args,
            project_root,
            quiet,
        )?;
    }

    Ok(())
}

// Define a helper struct to pass output-related args cleanly
// Made public so watch.rs can use it
pub struct OutputTargetArgs<'a> {
    pub save: &'a Option<Option<PathBuf>>,
    pub chunks: &'a Option<String>,
    pub stdout: bool,
    pub format_output: &'a crate::cli_args::FormatOutputOpts,
}

// Helper to get save details from OutputTargetArgs
fn get_save_details_from_args(
    config: &Config,
    cli_save_opt: Option<&Option<PathBuf>>,
    project_root: &Path,
) -> (PathBuf, String, String) {
    let save_dir_base = match cli_save_opt {
        Some(Some(cli_path)) => {
            log::trace!(
                "Save directory explicitly provided via CLI: {}",
                cli_path.display()
            );
            cli_path.clone()
        }
        Some(None) => {
            log::trace!(
                "Save flag used without path, using configured/default save directory: {}",
                config.save.output_dir.display()
            );
            config.save.output_dir.clone()
        }
        None => {
            log::trace!(
                "Save flag not used, using configured/default save directory for potential chunks: {}",
                config.save.output_dir.display()
            );
            config.save.output_dir.clone() // Default needed if chunking without -s
        }
    };

    let save_dir = if save_dir_base.is_absolute() {
        save_dir_base
    } else {
        project_root.join(save_dir_base)
    };
    log::trace!("Resolved absolute save directory: {}", save_dir.display());

    let name_options: [Option<&str>; 3] = [
        config.save.filename_base.as_deref(),
        config.general.project_name.as_deref(),
        project_root.file_name().and_then(|n| n.to_str()),
    ];
    let filename_base_str = name_options
        .into_iter()
        .flatten()
        .next()
        .unwrap_or("context"); // Fallback base name
    let filename_base = filename_base_str.to_string();
    log::trace!("Using filename base: {}", filename_base);

    let extension = config.save.extension.as_deref().unwrap_or_else(|| {
        match config.output.format.to_lowercase().as_str() {
            "yaml" | "yml" => "yaml",
            "xml" => "xml",
            _ => "json",
        }
    });
    log::trace!("Using save extension: {}", extension);

    (save_dir, filename_base, extension.to_string())
}

fn handle_final_output(
    main_context: &ProjectContext,
    config: &Config,
    output_target_args: &OutputTargetArgs,
    project_root: &Path,
    quiet: bool,
) -> Result<()> {
    log::debug!("Determining final output target...");
    let mut output_target_path: Option<PathBuf> = None;
    let needs_saving_to_disk = output_target_args.save.is_some();

    if needs_saving_to_disk {
        let (save_dir, filename_base, extension) =
            get_save_details_from_args(config, output_target_args.save.as_ref(), project_root);
        let main_filename = format!("{}.{}", filename_base, extension);
        output_target_path = Some(save_dir.join(main_filename));
        log::debug!(
            "Output target path set to file: {}",
            output_target_path.as_ref().unwrap().display()
        );
    } else if output_target_args.stdout {
        log::debug!("Output target set to stdout (forced).");
    } else {
        log::debug!("Output target set to stdout (default).");
    }

    output::print_context_or_save(
        main_context,
        config,
        output_target_path.as_deref(),
        &output_target_args.format_output,
        quiet,
    )
}

fn validate_args_for_generation(config: &Config, args: &OutputTargetArgs) -> Result<()> {
    if args.chunks.is_some() {
        if !config.source.enabled {
            anyhow::bail!(core::AppError::InvalidArgument(
                "Chunking (-c) cannot be used when source file inclusion ([source].enabled=false) is disabled".to_string()
            ));
        }
        let format = args
            .format_output
            .format
            .as_deref()
            .unwrap_or(&config.output.format);
        if format.to_lowercase() != "json" {
            anyhow::bail!(core::AppError::Chunking(
                "Chunking requires the output format to be 'json'. Use '-f json'.".to_string()
            ));
        }
        // Chunking implies saving, so stdout without save doesn't make sense unless explicitly handled
        if args.stdout && args.save.is_none() {
            anyhow::bail!(core::AppError::InvalidArgument(
                 "--stdout cannot be used with --chunks unless --save is also specified to define the main context output location.".to_string()
             ));
        }
    }
    Ok(())
}
