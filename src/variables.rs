use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::config::ConfigManager;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Variables {
    pub vars: HashMap<String, String>,
}

pub struct VariableManager {
    config_dir: PathBuf,
}

#[allow(dead_code)]
impl VariableManager {
    pub fn new(config_manager: &ConfigManager) -> Result<Self> {
        let config_dir = config_manager.config_dir().clone();
        let variables_dir = config_dir.join("variables");

        // Create variables directory if it doesn't exist
        fs::create_dir_all(&variables_dir).context("Failed to create variables directory")?;

        Ok(Self { config_dir })
    }

    pub fn set_variable(&mut self, context: &str, name: &str, value: &str) -> Result<()> {
        let mut variables = self.load_variables(context)?;
        variables.vars.insert(name.to_string(), value.to_string());
        self.save_variables(context, &variables)?;
        Ok(())
    }

    pub fn get_variable(&self, context: &str, name: &str) -> Result<Option<String>> {
        let variables = self.load_variables(context)?;
        Ok(variables.vars.get(name).cloned())
    }

    pub fn list_variables(&self, context: &str) -> Result<HashMap<String, String>> {
        let variables = self.load_variables(context)?;
        Ok(variables.vars)
    }

    pub fn remove_variable(&mut self, context: &str, name: &str) -> Result<bool> {
        let mut variables = self.load_variables(context)?;
        let removed = variables.vars.remove(name).is_some();
        if removed {
            self.save_variables(context, &variables)?;
        }
        Ok(removed)
    }

    pub fn resolve_variables(
        &self,
        context: &str,
        template: &str,
        args: &[String],
    ) -> Result<String> {
        let variables = self.load_variables(context)?;
        let mut resolved = template.to_string();

        // First, replace direct positional arguments ($1, $2, etc.) with provided args
        for (i, arg) in args.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            resolved = resolved.replace(&placeholder, arg);
        }

        // Then replace persistent variable placeholders like #1, #2, etc.
        for (name, value) in &variables.vars {
            resolved = resolved.replace(name, value);
        }

        // Finally, replace remaining positional arguments (#1, #2, etc.) with provided args
        // This handles cases where #1, #2 are not set as persistent variables
        let mut arg_index = 0;
        for i in 1..=args.len() + 10 {
            // Check up to 10 more placeholders than we have args
            let placeholder = format!("#{i}");
            if resolved.contains(&placeholder)
                && !variables.vars.contains_key(&placeholder)
                && arg_index < args.len()
            {
                resolved = resolved.replace(&placeholder, &args[arg_index]);
                arg_index += 1;
            }
        }

        Ok(resolved)
    }

    fn load_variables(&self, context: &str) -> Result<Variables> {
        let variables_file = self
            .config_dir
            .join("variables")
            .join(format!("{context}.yaml"));

        if variables_file.exists() {
            let contents =
                fs::read_to_string(&variables_file).context("Failed to read variables file")?;
            serde_yaml::from_str(&contents).context("Failed to parse variables file")
        } else {
            Ok(Variables::default())
        }
    }

    fn save_variables(&self, context: &str, variables: &Variables) -> Result<()> {
        let variables_file = self
            .config_dir
            .join("variables")
            .join(format!("{context}.yaml"));
        let yaml_content =
            serde_yaml::to_string(variables).context("Failed to serialize variables")?;
        fs::write(&variables_file, yaml_content).context("Failed to write variables file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigManager;
    use tempfile::TempDir;

    #[test]
    fn test_variable_operations() {
        let temp_dir = TempDir::new().unwrap();

        // Create a config manager with the temp directory
        let config_dir = temp_dir.path().join(".config").join("doo");
        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let mut variable_manager = VariableManager::new(&config_manager).unwrap();

        // Test setting and getting variables
        variable_manager
            .set_variable("test", "#1", "production")
            .unwrap();
        assert_eq!(
            variable_manager.get_variable("test", "#1").unwrap(),
            Some("production".to_string())
        );

        // Test variable resolution
        let resolved = variable_manager
            .resolve_variables("test", "kubectl get pods -n #1", &[])
            .unwrap();
        assert_eq!(resolved, "kubectl get pods -n production");
    }

    #[test]
    fn test_positional_args() {
        let temp_dir = TempDir::new().unwrap();

        let config_dir = temp_dir.path().join(".config").join("doo");
        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let variable_manager = VariableManager::new(&config_manager).unwrap();

        let resolved = variable_manager
            .resolve_variables(
                "default",
                "kubectl logs #1 #2",
                &["pod-name".to_string(), "-f".to_string()],
            )
            .unwrap();
        assert_eq!(resolved, "kubectl logs pod-name -f");
    }

    #[test]
    fn test_dollar_positional_args() {
        let temp_dir = TempDir::new().unwrap();

        let config_dir = temp_dir.path().join(".config").join("doo");
        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let variable_manager = VariableManager::new(&config_manager).unwrap();

        // Test $1, $2 direct positional argument replacement
        let resolved = variable_manager
            .resolve_variables(
                "default",
                "kubectl -n $1 get pods",
                &["console".to_string()],
            )
            .unwrap();
        assert_eq!(resolved, "kubectl -n console get pods");

        // Test multiple $1, $2 arguments
        let resolved = variable_manager
            .resolve_variables(
                "default",
                "kubectl logs $1 -n $2",
                &["my-pod".to_string(), "staging".to_string()],
            )
            .unwrap();
        assert_eq!(resolved, "kubectl logs my-pod -n staging");
    }

    #[test]
    fn test_mixed_placeholders() {
        let temp_dir = TempDir::new().unwrap();

        let config_dir = temp_dir.path().join(".config").join("doo");
        let config_manager = ConfigManager::new_with_dir(config_dir).unwrap();
        let mut variable_manager = VariableManager::new(&config_manager).unwrap();

        // Set a persistent variable #1
        variable_manager
            .set_variable("test", "#1", "production")
            .unwrap();

        // Test mixing persistent #1 with direct $1 arguments
        let resolved = variable_manager
            .resolve_variables(
                "test",
                "kubectl -n #1 get pods $1",
                &["--watch".to_string()],
            )
            .unwrap();
        assert_eq!(resolved, "kubectl -n production get pods --watch");
    }
}
