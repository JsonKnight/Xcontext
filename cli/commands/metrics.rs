use crate::cli_args::MetricsArgs;
use crate::load_config_for_command;
use crate::output::{print_data_or_text, print_metrics_pretty_table};
use anyhow::{Context, Result};
use byte_unit::{Byte, UnitType};
use log;
use pathdiff; // Added use
use serde::Serialize;
use std::path::Path;
use tiktoken_rs::cl100k_base;
use xcontext_core::{self as core, Config, FileInfo}; // Use core types

#[derive(Debug, Serialize)]
pub struct ProjectMetrics {
    pub total_files: usize,
    pub total_lines: usize,
    pub total_bytes: u128,
    pub total_bytes_readable: String,
    pub estimated_tokens: usize,
    pub files_details: Vec<FileMetrics>,
}

#[derive(Debug, Serialize)]
pub struct FileMetrics {
    pub path: String,
    pub lines: usize,
    pub bytes: usize,
    pub bytes_readable: String,
    pub estimated_tokens: usize,
}

pub fn handle_metrics_command(args: MetricsArgs, quiet: bool) -> Result<()> {
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
    .context("Failed to load configuration for metrics command")?;

    log::debug!("Gathering files for metrics...");
    let (source_files, docs_files, _) = core::gather_files_and_tree(&project_root, &config, quiet)
        .context("Failed to gather files for metrics calculation")?;
    log::debug!("Files gathered.");

    let combined_files: Vec<&FileInfo> = source_files.iter().chain(docs_files.iter()).collect();

    if combined_files.is_empty() && !quiet {
        println!("No source or documentation files found to calculate metrics.");
        return Ok(()); // Exit gracefully if no files
    }

    log::debug!("Calculating metrics...");
    let metrics = calculate_metrics(&combined_files, &project_root)?;
    log::debug!("Metrics calculation complete.");

    if args.format_output.format.is_none() {
        print_metrics_pretty_table(&metrics)
    } else {
        // Pass None for plain_text, rely on structured output
        print_data_or_text(
            &metrics,
            None,
            &args.format_output,
            "json",
            "ProjectMetrics",
        )
    }
}

fn calculate_metrics(files: &[&FileInfo], project_root: &Path) -> Result<ProjectMetrics> {
    let bpe =
        cl100k_base().map_err(|e| anyhow::anyhow!(core::AppError::TikToken(e.to_string())))?;
    let mut total_files = 0;
    let mut total_lines = 0;
    let mut total_bytes: u128 = 0;
    let mut total_tokens = 0;
    let mut files_details = Vec::new();

    for file_info in files {
        if file_info.size == 0 {
            continue;
        } // Skip empty files

        let lines = file_info.content.lines().count();
        let bytes = file_info.size;
        // Estimate tokens in parallel? Might be overkill unless content is huge
        let tokens = bpe.encode_ordinary(&file_info.content).len();

        let relative_path = pathdiff::diff_paths(&file_info.path, project_root) // Added use pathdiff
            .unwrap_or_else(|| file_info.path.clone())
            .to_string_lossy()
            .to_string();

        total_files += 1;
        total_lines += lines;
        total_bytes = total_bytes.saturating_add(bytes as u128);
        total_tokens += tokens;

        let file_byte = Byte::from_u128(bytes as u128).unwrap_or_default();
        let file_size_readable = file_byte.get_appropriate_unit(UnitType::Binary).to_string();

        files_details.push(FileMetrics {
            path: relative_path,
            lines,
            bytes,
            bytes_readable: file_size_readable,
            estimated_tokens: tokens,
        });
    }

    // Sort by path for consistent output
    files_details.sort_by(|a, b| a.path.cmp(&b.path));

    let total_byte = Byte::from_u128(total_bytes).unwrap_or_default();
    let total_size_readable = total_byte
        .get_appropriate_unit(UnitType::Binary)
        .to_string();

    Ok(ProjectMetrics {
        total_files,
        total_lines,
        total_bytes,
        total_bytes_readable: total_size_readable,
        estimated_tokens: total_tokens,
        files_details,
    })
}
