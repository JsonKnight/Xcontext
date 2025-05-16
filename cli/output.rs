use anyhow::{Context, Result};
use colored::*;
// Corrected: Separate use statements onto different lines
use comfy_table::{Cell, Color, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use xcontext_core::{ChunkFile, ProjectContext, output_formats}; // Use core types

use crate::cli_args::FormatOutputOpts; // Use CLI format options

// --- Public Output Functions ---

pub fn print_context_or_save(
    context: &ProjectContext,
    config: &xcontext_core::Config, // Use core config
    output_path: Option<&Path>,
    format_opts: &FormatOutputOpts,
    quiet: bool,
) -> Result<()> {
    let final_format = format_opts
        .format
        .as_deref()
        .unwrap_or(&config.output.format);
    let pretty_json = !config.output.json_minify; // Use config value after overrides
    let pretty_xml = config.output.xml_pretty_print; // Use config value after overrides

    let content = serialize_output(
        context,
        final_format,
        pretty_json,
        pretty_xml,
        "ProjectContext",
    )?;

    match output_path {
        Some(path) => {
            write_to_file(path, &content)?;
            let is_chunked = context.source.as_ref().is_some_and(|s| s.chunks.is_some());
            if !is_chunked && !quiet {
                println!(
                    "{} Context saved to: {}",
                    "✅".green(),
                    path.display().to_string().blue()
                );
            }
        }
        None => {
            write_to_stdout(&content)?;
        }
    }
    Ok(())
}

pub fn save_chunk_file(
    chunk_data: &ChunkFile,
    path: &Path,
    format_opts: &FormatOutputOpts, // Use CLI format opts for chunk format
    quiet: bool,
) -> Result<()> {
    // Chunks are always JSON for now, respect pretty/minify from CLI args
    let pretty = !format_opts.disable_json_minify || format_opts.enable_json_minify;
    let content = output_formats::serialize_context_to_json(chunk_data, pretty)?;

    write_to_file(path, &content)?;
    if !quiet {
        println!(
            "{} Chunk saved to: {}",
            "📦".blue(), // Use different emoji for chunks
            path.display().to_string().dimmed()
        );
    }
    Ok(())
}

// Helper for commands that might output structured data or plain text
pub fn print_data_or_text<T: Serialize>(
    data: &T,
    plain_text: Option<String>,
    format_opts: &FormatOutputOpts,
    default_format: &str, // e.g., "json" or "text"
    root_name: &str,      // For XML root element
) -> Result<()> {
    let format = format_opts
        .format
        .as_deref()
        .unwrap_or(default_format)
        .to_lowercase();

    if format == "text" {
        match plain_text {
            Some(text) => write_to_stdout(&text),
            None => {
                // Fallback to JSON pretty print if text is not available but format is text
                let pretty = true;
                let content = output_formats::serialize_context_to_json(data, pretty)?;
                write_to_stdout(&content)
            }
        }
    } else {
        let pretty_json = !format_opts.disable_json_minify;
        let pretty_xml = format_opts.enable_xml_pretty;
        let content = serialize_output(data, &format, pretty_json, pretty_xml, root_name)?;
        write_to_stdout(&content)
    }
}

// --- Internal Helpers ---

fn serialize_output<T: Serialize>(
    data: &T,
    format: &str,
    pretty_json: bool,
    pretty_xml: bool,
    xml_root: &str,
) -> Result<String> {
    match format.to_lowercase().as_str() {
        "yaml" | "yml" => {
            output_formats::serialize_context_to_yaml(data).map_err(anyhow::Error::from)
        }
        "xml" => output_formats::serialize_context_to_xml(data, xml_root, pretty_xml)
            .map_err(anyhow::Error::from),
        "json" | _ => {
            // Default to JSON
            output_formats::serialize_context_to_json(data, pretty_json)
                .map_err(anyhow::Error::from)
        }
    }
}

fn write_to_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        // Added Context
    }
    let mut file =
        File::create(path).with_context(|| format!("Failed to create file {}", path.display()))?; // Added Context
    file.write_all(content.as_bytes())
        .with_context(|| format!("Failed to write to file {}", path.display()))?; // Added Context
    Ok(())
}

fn write_to_stdout(content: &str) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(content.as_bytes())
        .context("Failed to write to stdout")?; // Added Context
    // Add a newline if the content doesn't end with one, for better terminal behavior
    // Corrected: Check for actual newline character '\n'
    if !content.ends_with('\n') {
        handle
            .write_all(b"\\n") // Still write literal `\n` if adding one, common practice
            .context("Failed to write newline to stdout")?; // Added Context
    }
    handle.flush().context("Failed to flush stdout")?; // Added Context
    Ok(())
}

// Example of a pretty printer function for a specific command (e.g., metrics)
pub fn print_metrics_pretty_table(
    metrics: &crate::commands::metrics::ProjectMetrics,
) -> Result<()> {
    println!();
    println!("{}", " Project Metrics Summary ".green().bold().underline());
    println!(
        "{:<20} {}",
        "Total Files:".green(),
        metrics.total_files.to_string().cyan()
    );
    println!(
        "{:<20} {}",
        "Total Lines:".green(),
        metrics.total_lines.to_string().cyan()
    );
    println!(
        "{:<20} {}",
        "Total Size:".green(),
        metrics.total_bytes_readable.cyan()
    );
    println!(
        "{:<20} {}",
        "Est. Tokens:".green(),
        metrics.estimated_tokens.to_string().cyan()
    );

    if metrics.files_details.is_empty() {
        println!("\n{}", "(No files included in metrics)".yellow());
    } else {
        println!("\n{}", " File Details ".green().bold().underline());
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);
        table.set_header(vec![
            Cell::new("Path").fg(Color::Green),
            Cell::new("Lines").fg(Color::Green),
            Cell::new("Size").fg(Color::Green),
            Cell::new("Tokens").fg(Color::Green),
        ]);
        for file in &metrics.files_details {
            table.add_row(vec![
                Cell::new(&file.path).fg(Color::Cyan),
                Cell::new(file.lines).set_alignment(comfy_table::CellAlignment::Right),
                Cell::new(&file.bytes_readable)
                    .set_alignment(comfy_table::CellAlignment::Right)
                    .fg(Color::DarkGrey),
                Cell::new(file.estimated_tokens).set_alignment(comfy_table::CellAlignment::Right),
            ]);
        }
        println!("{table}");
    }
    println!();
    Ok(())
}
