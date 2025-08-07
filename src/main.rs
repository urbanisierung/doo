use anyhow::Result;
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::process;

mod config;
mod context;
mod executor;
mod interactive;
mod variables;

use config::ConfigManager;
use context::ContextManager;
use executor::CommandExecutor;
use interactive::InteractiveMenu;
use variables::VariableManager;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let matches = build_cli().get_matches();

    // Initialize managers
    let mut config_manager = ConfigManager::new()?;
    let mut context_manager = ContextManager::new(&config_manager)?;
    let mut variable_manager = VariableManager::new(&config_manager)?;
    let executor = CommandExecutor::new();

    match matches.subcommand() {
        Some(("var", sub_matches)) => {
            handle_variable_command(sub_matches, &mut variable_manager, &context_manager)?;
        }
        Some(("context", sub_matches)) => {
            handle_context_command(sub_matches, &mut context_manager)?;
        }
        Some(("import", sub_matches)) => {
            handle_import_command(sub_matches, &mut config_manager).await?;
        }
        Some(("import-repo", sub_matches)) => {
            handle_import_repo_command(sub_matches, &mut config_manager).await?;
        }
        Some(("sync", _)) => {
            handle_sync_command(&mut config_manager).await?;
        }
        Some((cmd_name, _)) => {
            // For external subcommands, collect all trailing arguments
            let raw_args = std::env::args().collect::<Vec<_>>();
            let mut args = Vec::new();

            // Find the position after the command name and collect all following args
            if let Some(cmd_pos) = raw_args.iter().position(|arg| arg == cmd_name) {
                args = raw_args.into_iter().skip(cmd_pos + 1).collect();
            }

            handle_command_execution(
                cmd_name,
                args,
                &mut config_manager,
                &variable_manager,
                &context_manager,
                &executor,
            )?;
        }
        None => {
            // No subcommand provided, show interactive menu
            let menu = InteractiveMenu::new(&config_manager, &variable_manager, &context_manager)?;
            menu.run(&executor)?;
        }
    }

    Ok(())
}

fn build_cli() -> Command {
    Command::new("doo")
        .about("A CLI wrapper for other commands with persistent variables and contexts")
        .version("0.1.0")
        .author("Your Name")
        .arg_required_else_help(false)
        .subcommand(
            Command::new("var")
                .about("Manage variables")
                .arg(
                    Arg::new("name")
                        .help("Variable name (e.g., #1)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("value")
                        .help("Variable value")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            Command::new("context").about("Switch context").arg(
                Arg::new("name")
                    .help("Context name")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            Command::new("import")
                .about("Import a config file from local path or GitHub repository")
                .arg(
                    Arg::new("file")
                        .help("Path to config file or GitHub repository (owner/repo)")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("import-repo")
                .about("Import all YAML config files from a GitHub repository")
                .arg(
                    Arg::new("repo")
                        .help("GitHub repository (owner/repo)")
                        .required(true)
                        .index(1),
                ),
        )
        .subcommand(
            Command::new("sync").about("Sync all imported configs with their remote origins"),
        )
        .allow_external_subcommands(true)
}

fn handle_variable_command(
    matches: &ArgMatches,
    variable_manager: &mut VariableManager,
    context_manager: &ContextManager,
) -> Result<()> {
    let name = matches.get_one::<String>("name").unwrap();
    let value = matches.get_one::<String>("value").unwrap();

    variable_manager.set_variable(context_manager.current_context(), name, value)?;
    println!(
        "{} Variable {} set to {} in context {}",
        "✓".green().bold(),
        name.cyan().bold(),
        value.yellow(),
        context_manager.current_context().blue().bold()
    );

    Ok(())
}

fn handle_context_command(
    matches: &ArgMatches,
    context_manager: &mut ContextManager,
) -> Result<()> {
    let context_name = matches.get_one::<String>("name").unwrap();
    context_manager.switch_context(context_name)?;
    println!(
        "{} Switched to context {}",
        "✓".green().bold(),
        context_name.blue().bold()
    );

    Ok(())
}

async fn handle_import_command(
    matches: &ArgMatches,
    config_manager: &mut ConfigManager,
) -> Result<()> {
    let file_path = matches.get_one::<String>("file").unwrap();

    // Check if it's a GitHub repository (contains /)
    if file_path.contains('/') && !file_path.contains('.') && !file_path.starts_with('/') {
        // GitHub repository format: owner/repo
        match config_manager.import_config_from_github(file_path).await {
            Ok(imported_name) => {
                println!(
                    "{} Successfully imported config from GitHub repository '{}' as '{}'",
                    "✓".green().bold(),
                    file_path.cyan().bold(),
                    imported_name.cyan().bold()
                );
            }
            Err(e) => {
                println!(
                    "{} Failed to import from GitHub repository '{}': {}",
                    "✗".red().bold(),
                    file_path.yellow(),
                    e.to_string().red()
                );
                process::exit(1);
            }
        }
    } else {
        // Local file import
        match config_manager.import_config(file_path) {
            Ok(imported_name) => {
                println!(
                    "{} Successfully imported config file as '{}'",
                    "✓".green().bold(),
                    imported_name.cyan().bold()
                );
            }
            Err(e) => {
                println!(
                    "{} Failed to import config file: {}",
                    "✗".red().bold(),
                    e.to_string().red()
                );
                process::exit(1);
            }
        }
    }

    Ok(())
}

async fn handle_import_repo_command(
    matches: &ArgMatches,
    config_manager: &mut ConfigManager,
) -> Result<()> {
    let repo = matches.get_one::<String>("repo").unwrap();

    match config_manager.import_repo_configs(repo).await {
        Ok(imported_configs) => {
            println!(
                "{} Successfully imported {} config file(s) from repository '{}':",
                "✓".green().bold(),
                imported_configs.len(),
                repo.cyan().bold()
            );
            for config_name in imported_configs {
                println!("  • {}", config_name.cyan());
            }
        }
        Err(e) => {
            println!(
                "{} Failed to import repository '{}': {}",
                "✗".red().bold(),
                repo.yellow(),
                e.to_string().red()
            );
            process::exit(1);
        }
    }

    Ok(())
}

async fn handle_sync_command(config_manager: &mut ConfigManager) -> Result<()> {
    match config_manager.sync_all_configs().await {
        Ok(()) => {
            // Success message is already printed in sync_all_configs
        }
        Err(e) => {
            println!(
                "{} Failed to sync configs: {}",
                "✗".red().bold(),
                e.to_string().red()
            );
            process::exit(1);
        }
    }

    Ok(())
}

fn handle_command_execution(
    cmd_name: &str,
    args: Vec<String>,
    config_manager: &mut ConfigManager,
    variable_manager: &VariableManager,
    context_manager: &ContextManager,
    executor: &CommandExecutor,
) -> Result<()> {
    // Check for conflicts first
    let conflicts = config_manager.get_command_conflicts(cmd_name);

    if conflicts.is_empty() {
        println!(
            "{} Command '{}' not found. Use 'doo' without arguments to browse available commands.",
            "✗".red().bold(),
            cmd_name.yellow()
        );
        process::exit(1);
    }

    let command_template = if conflicts.len() == 1 {
        // No conflict, use the single command
        conflicts[0].command.clone()
    } else {
        // Multiple definitions found, ask user to choose
        println!(
            "{} Command '{}' found in multiple config files:",
            "⚠".yellow().bold(),
            cmd_name.cyan().bold()
        );

        for (i, conflict) in conflicts.iter().enumerate() {
            println!(
                "  {}) {} (from {}): {}",
                i + 1,
                cmd_name.cyan(),
                conflict.source_file.blue(),
                conflict.command.bright_white()
            );
        }

        print!(
            "\nWhich config file should be used? Enter number (1-{}): ",
            conflicts.len()
        );
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        match input.trim().parse::<usize>() {
            Ok(choice) if choice >= 1 && choice <= conflicts.len() => {
                conflicts[choice - 1].command.clone()
            }
            _ => {
                println!("{} Invalid choice", "✗".red().bold());
                process::exit(1);
            }
        }
    };

    let resolved_command = variable_manager.resolve_variables(
        context_manager.current_context(),
        &command_template,
        &args,
    )?;

    println!(
        "{} {}",
        "Executing:".green().bold(),
        resolved_command.bright_white()
    );

    executor.execute(&resolved_command)?;

    Ok(())
}
