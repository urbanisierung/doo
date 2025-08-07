use anyhow::Result;
use colored::*;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};

use crate::config::ConfigManager;
use crate::context::ContextManager;
use crate::executor::CommandExecutor;
use crate::variables::VariableManager;

pub struct InteractiveMenu<'a> {
    config_manager: &'a ConfigManager,
    variable_manager: &'a VariableManager,
    context_manager: &'a ContextManager,
}

impl<'a> InteractiveMenu<'a> {
    pub fn new(
        config_manager: &'a ConfigManager,
        variable_manager: &'a VariableManager,
        context_manager: &'a ContextManager,
    ) -> Result<Self> {
        Ok(Self {
            config_manager,
            variable_manager,
            context_manager,
        })
    }

    #[allow(clippy::never_loop)]
    pub fn run(&self, executor: &CommandExecutor) -> Result<()> {
        loop {
            // Get all commands
            let commands = self.config_manager.search_commands("");
            if commands.is_empty() {
                println!("{}", "No commands available.".red());
                return Ok(());
            }

            // Prepare command list with better formatting for better visual distinction
            let command_items: Vec<String> = commands
                .iter()
                .map(|(name, template)| {
                    // Use simple but clear formatting without ANSI codes
                    format!("[{name}]  =>  {template}")
                })
                .collect();

            // Show context information
            println!();
            println!("{}", "┌─ DOO Command Browser ─┐".cyan().bold());
            println!(
                "│ Context: {} │", 
                self.context_manager.current_context().blue().bold()
            );
            println!("{}", "└─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─┘".cyan());

            // Use dialoguer's FuzzySelect for the interactive menu
            let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Search and select command")
                .default(0)
                .items(&command_items)
                .interact_opt()?;

            match selection {
                Some(index) => {
                    let (cmd_name, cmd_template) = &commands[index];
                    
                    // Clear screen for cleaner output
                    print!("\x1B[2J\x1B[1;1H");
                    
                    // Execute the selected command
                    self.execute_selected_command(cmd_name, cmd_template, executor)?;
                    
                    return Ok(());
                }
                None => {
                    // User pressed Escape - clear screen and exit
                    print!("\x1B[2J\x1B[1;1H");
                    return Ok(());
                }
            }
        }
    }

    fn execute_selected_command(
        &self,
        cmd_name: &str,
        cmd_template: &str,
        executor: &CommandExecutor,
    ) -> Result<()> {
        println!(
            "{} Selected command: {}",
            "✓".green().bold(),
            cmd_name.cyan().bold()
        );
        
        // Check for conflicts before executing
        let conflicts = self.config_manager.get_command_conflicts(cmd_name);
        
        let final_template = if conflicts.len() > 1 {
            // Multiple definitions found, ask user to choose
            println!(
                "{} Command '{}' found in multiple config files:",
                "⚠".yellow().bold(),
                cmd_name.cyan().bold()
            );
            
            let options: Vec<String> = conflicts
                .iter()
                .map(|conflict| format!("{} ({})", conflict.source_file, conflict.command))
                .collect();
            
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Which config file should be used?")
                .default(0)
                .items(&options)
                .interact()?;
            
            conflicts[selection].command.clone()
        } else {
            cmd_template.to_string()
        };

        // Resolve variables in the command template
        let resolved_command = self.variable_manager.resolve_variables(
            self.context_manager.current_context(),
            &final_template,
            &[],
        )?;

        println!(
            "{} {}",
            "Executing:".green().bold(),
            resolved_command.bright_white()
        );
        
        executor.execute(&resolved_command)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;
    use crate::context::ContextManager;
    use crate::variables::VariableManager;
    use tempfile::TempDir;

    #[test]
    fn test_interactive_menu_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config").join("doo");
        
        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let context_manager = ContextManager::new(&config_manager).unwrap();
        let variable_manager = VariableManager::new(&config_manager).unwrap();
        
        let menu = InteractiveMenu::new(&config_manager, &variable_manager, &context_manager);
        assert!(menu.is_ok());
    }
}
