use anyhow::Result;
use doo::{ConfigManager, ContextManager, VariableManager};
use tempfile::TempDir;

#[test]
fn test_full_workflow() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join(".config").join("doo");

    // Initialize managers
    let config_manager = ConfigManager::new_with_dir(config_dir)?;
    let mut context_manager = ContextManager::new(&config_manager)?;
    let mut variable_manager = VariableManager::new(&config_manager)?;

    // Test context switching
    assert_eq!(context_manager.current_context(), "default");
    context_manager.switch_context("test")?;
    assert_eq!(context_manager.current_context(), "test");

    // Test variable management
    variable_manager.set_variable("test", "#1", "production")?;
    assert_eq!(
        variable_manager.get_variable("test", "#1")?,
        Some("production".to_string())
    );

    // Test command resolution
    let resolved = variable_manager.resolve_variables("test", "kubectl get pods -n #1", &[])?;
    assert_eq!(resolved, "kubectl get pods -n production");

    // Test with positional arguments
    let resolved = variable_manager.resolve_variables(
        "test",
        "kubectl logs #1 #2",
        &["my-pod".to_string()],
    )?;
    assert_eq!(resolved, "kubectl logs production my-pod");

    Ok(())
}

#[test]
fn test_command_management() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join(".config").join("doo");

    let mut config_manager = ConfigManager::new_with_dir(config_dir)?;

    // Test adding custom command
    config_manager.add_command("custom", "echo #1")?;
    assert_eq!(
        config_manager.get_command("custom")?,
        Some("echo #1".to_string())
    );

    // Test command search
    let results = config_manager.search_commands("echo");
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.name == "custom"));

    Ok(())
}

#[test]
fn test_context_isolation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join(".config").join("doo");

    let config_manager = ConfigManager::new_with_dir(config_dir)?;
    let mut context_manager = ContextManager::new(&config_manager)?;
    let mut variable_manager = VariableManager::new(&config_manager)?;

    // Set variable in default context
    variable_manager.set_variable("default", "#1", "default-value")?;

    // Switch to test context and set different variable
    context_manager.switch_context("test")?;
    variable_manager.set_variable("test", "#1", "test-value")?;

    // Verify isolation
    assert_eq!(
        variable_manager.get_variable("default", "#1")?,
        Some("default-value".to_string())
    );
    assert_eq!(
        variable_manager.get_variable("test", "#1")?,
        Some("test-value".to_string())
    );

    Ok(())
}
