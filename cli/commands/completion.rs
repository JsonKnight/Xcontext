use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use colored::*;
use std::fs::{self, File};
use std::io::{self, Write};
use xcontext_core::AppError; // Use core error for specific cases if needed

use crate::cli_args::{Cli, CompletionArgs};

pub fn handle_completion_command(args: &CompletionArgs, quiet: bool) -> Result<()> {
    let shell_str = args.shell.as_deref().unwrap_or("fish");
    let save_output = args.save;

    let shell_enum: Shell = match shell_str.to_lowercase().as_str() {
        "fish" => Shell::Fish,
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        _ => {
            anyhow::bail!(AppError::InvalidArgument(format!(
                // Use anyhow::bail! for CLI errors
                "Unsupported shell for completion: {}",
                shell_str
            )));
        }
    };

    let mut command = Cli::command();
    let bin_name = command.get_name().to_string();

    if !save_output {
        generate(shell_enum, &mut command, bin_name, &mut io::stdout());
    } else {
        let save_dir_res = match shell_enum {
            Shell::Fish => dirs::config_dir().map(|p| p.join("fish").join("completions")),
            Shell::Bash => dirs::config_dir().map(|p| p.join("bash_completion.d")), // Common location
            Shell::Zsh => dirs::data_local_dir().map(|p| p.join("zsh").join("site-functions")),
            _ => anyhow::bail!(AppError::InvalidArgument(format!(
                "Default save location not known for shell: {}",
                shell_str
            ))),
        };

        let save_dir = save_dir_res
            .ok_or_else(|| anyhow::anyhow!("Could not determine standard completion directory."))?;

        let filename = match shell_enum {
            Shell::Fish => format!("{}.fish", bin_name),
            Shell::Bash => format!("{}.bash", bin_name), // Or just bin_name
            Shell::Zsh => format!("_{}", bin_name),
            _ => unreachable!(),
        };
        let save_path = save_dir.join(&filename); // Use reference

        if save_path.exists() {
            if !quiet {
                print!(
                    "{} Completion file already exists at '{}'. Overwrite? [{}/{}] ",
                    "⚠️".yellow(),
                    save_path.display().to_string().cyan(),
                    "y".green(),
                    "N".red()
                );
                io::stdout().flush().context("Failed to flush stdout")?;
                let mut response = String::new();
                io::stdin()
                    .read_line(&mut response)
                    .context("Failed to read user input")?;
                if !response.trim().eq_ignore_ascii_case("y") {
                    println!("Save cancelled.");
                    return Ok(());
                }
            } else {
                anyhow::bail!(
                    "Target file '{}' exists. Overwrite prevented in quiet mode.",
                    save_path.display()
                );
            }
        }

        fs::create_dir_all(&save_dir)
            .with_context(|| format!("Failed to create directory {}", save_dir.display()))?;
        let mut file = File::create(&save_path)
            .with_context(|| format!("Failed to create file {}", save_path.display()))?;
        generate(shell_enum, &mut command, bin_name, &mut file);

        if !quiet {
            println!(
                "{} {} completions saved to: {}",
                "✅".green(),
                shell_str.cyan(),
                save_path.display().to_string().blue()
            );
        }
    }
    Ok(())
}
