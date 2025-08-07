use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::config::ConfigManager;

pub struct ContextManager {
    config_dir: PathBuf,
    current_context: String,
}

#[allow(dead_code)]
impl ContextManager {
    pub fn new(config_manager: &ConfigManager) -> Result<Self> {
        let config_dir = config_manager.config_dir().clone();
        let context_file = config_dir.join("current_context");

        let current_context = if context_file.exists() {
            fs::read_to_string(&context_file)
                .context("Failed to read current context file")?
                .trim()
                .to_string()
        } else {
            "default".to_string()
        };

        Ok(Self {
            config_dir,
            current_context,
        })
    }

    pub fn current_context(&self) -> &str {
        &self.current_context
    }

    pub fn switch_context(&mut self, context: &str) -> Result<()> {
        self.current_context = context.to_string();
        let context_file = self.config_dir.join("current_context");
        fs::write(&context_file, &self.current_context)
            .context("Failed to write current context file")?;
        Ok(())
    }

    pub fn list_contexts(&self) -> Result<Vec<String>> {
        let variables_dir = self.config_dir.join("variables");
        if !variables_dir.exists() {
            return Ok(vec!["default".to_string()]);
        }

        let mut contexts = vec!["default".to_string()];

        for entry in fs::read_dir(&variables_dir).context("Failed to read variables directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".yaml") {
                    let context_name = name.strip_suffix(".yaml").unwrap();
                    if context_name != "default" {
                        contexts.push(context_name.to_string());
                    }
                }
            }
        }

        contexts.sort();
        Ok(contexts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;
    use tempfile::TempDir;

    #[test]
    fn test_context_switching() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config").join("doo");

        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let mut context_manager = ContextManager::new(&config_manager).unwrap();

        assert_eq!(context_manager.current_context(), "default");

        context_manager.switch_context("test").unwrap();
        assert_eq!(context_manager.current_context(), "test");
    }
}
