use anyhow::{Context, Result};
use colored::*;
use std::process::{Command, Stdio};

pub struct CommandExecutor;

#[allow(dead_code)]
impl CommandExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self, command_line: &str) -> Result<()> {
        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let command = parts[0];
        let args = &parts[1..];

        println!("{}", "─".repeat(50).bright_black());

        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .with_context(|| format!("Failed to execute command: {command}"))?;

        let status = child.wait()
            .with_context(|| format!("Failed to wait for command: {command}"))?;

        println!("{}", "─".repeat(50).bright_black());

        if !status.success() {
            if let Some(code) = status.code() {
                println!(
                    "{} Command exited with code {}",
                    "✗".red().bold(),
                    code.to_string().red()
                );
            } else {
                println!("{} Command was terminated by signal", "✗".red().bold());
            }
        } else {
            println!("{} Command completed successfully", "✓".green().bold());
        }

        Ok(())
    }

    pub fn execute_with_output(&self, command_line: &str) -> Result<String> {
        let parts: Vec<&str> = command_line.split_whitespace().collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Empty command"));
        }

        let command = parts[0];
        let args = &parts[1..];

        let output = Command::new(command)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute command: {command}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Command failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.to_string())
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_command() {
        let executor = CommandExecutor::new();
        let result = executor.execute_with_output("echo hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "hello");
    }

    #[test]
    fn test_execute_invalid_command() {
        let executor = CommandExecutor::new();
        let result = executor.execute_with_output("nonexistent_command_12345");
        assert!(result.is_err());
    }
}
