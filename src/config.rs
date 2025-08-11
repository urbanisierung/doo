use anyhow::{anyhow, Context, Result};
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub commands: HashMap<String, CommandEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ConfigOrigin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CommandEntry {
    /// Simple string form: name: "command template"
    Simple(String),
    /// Detailed form with optional description used for search & interactive menu display
    Detailed {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

impl CommandEntry {
    pub fn command_str(&self) -> &str {
        match self {
            CommandEntry::Simple(s) => s,
            CommandEntry::Detailed { command, .. } => command,
        }
    }
    pub fn description(&self) -> Option<&str> {
        match self {
            CommandEntry::Simple(_) => None,
            CommandEntry::Detailed { description, .. } => description.as_deref(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigOrigin {
    pub repo: String, // owner/repo format
    pub import_type: ImportType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ImportType {
    Public,
    Private,
}

#[derive(Debug, Clone)]
pub struct CommandSource {
    #[allow(dead_code)]
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub source_file: String,
}

#[derive(Debug, Clone)]
pub struct CommandSearchResult {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubContent {
    #[allow(dead_code)]
    name: String,
    content: String,
    encoding: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRepo {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    description: Option<String>,
}

pub struct ConfigManager {
    config_dir: PathBuf,
    configs_dir: PathBuf,
    main_config: Config,
    imported_configs: HashMap<String, Config>,
}

#[allow(dead_code)]
impl ConfigManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("doo");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        // Create configs subdirectory for imported configs
        let configs_dir = config_dir.join("configs");
        fs::create_dir_all(&configs_dir).context("Failed to create configs directory")?;

        let config_file = config_dir.join("config.yaml");
        let main_config = if config_file.exists() {
            let contents =
                fs::read_to_string(&config_file).context("Failed to read config file")?;
            serde_yaml::from_str(&contents).context("Failed to parse config file")?
        } else {
            // Create default config with some examples
            let default_config = Config {
                commands: HashMap::from([
                    (
                        "watch".to_string(),
                        CommandEntry::Detailed {
                            command: "watch kubectl -n #1 get pods".to_string(),
                            description: Some("Watch pods in current namespace (#1)".to_string()),
                        },
                    ),
                    (
                        "logs".to_string(),
                        CommandEntry::Simple("kubectl logs -f -n #1 #2".to_string()),
                    ),
                    (
                        "pods".to_string(),
                        CommandEntry::Simple("kubectl get pods -n #1".to_string()),
                    ),
                    (
                        "describe".to_string(),
                        CommandEntry::Simple("kubectl describe pod -n #1 #2".to_string()),
                    ),
                ]),
                origin: None, // Main config has no origin
            };

            let yaml_content = serde_yaml::to_string(&default_config)
                .context("Failed to serialize default config")?;
            fs::write(&config_file, yaml_content).context("Failed to write default config file")?;

            default_config
        };

        // Load all imported configs from files and repository directories
        let mut imported_configs = HashMap::new();

        // Load configs from files in configs directory
        for entry in fs::read_dir(&configs_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == "yaml" || ext == "yml")
            {
                let file_name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .context("Invalid config file name")?
                    .to_string();

                let contents = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config file: {path:?}"))?;
                let config: Config = serde_yaml::from_str(&contents)
                    .with_context(|| format!("Failed to parse config file: {path:?}"))?;

                imported_configs.insert(file_name, config);
            }
        }

        // Load configs from repository directories
        for entry in fs::read_dir(&configs_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                // This is a repository directory, scan for YAML files
                for repo_entry in fs::read_dir(&path)? {
                    let repo_entry = repo_entry?;
                    let repo_file_path = repo_entry.path();

                    if repo_file_path.is_file()
                        && repo_file_path
                            .extension()
                            .is_some_and(|ext| ext == "yaml" || ext == "yml")
                    {
                        let repo_name = path.file_name().unwrap().to_str().unwrap();
                        let file_stem = repo_file_path
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .unwrap_or("config");

                        // Create unique config name: repo_filename
                        let config_name = format!("{repo_name}_{file_stem}");

                        let contents = fs::read_to_string(&repo_file_path).with_context(|| {
                            format!("Failed to read repo config file: {repo_file_path:?}")
                        })?;

                        if let Ok(config) = serde_yaml::from_str::<Config>(&contents) {
                            // Only add if it's a valid doo config with commands
                            if !config.commands.is_empty() {
                                imported_configs.insert(config_name, config);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            config_dir,
            configs_dir,
            main_config,
            imported_configs,
        })
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    pub fn import_config(&mut self, source_path: &str) -> Result<String> {
        let source_path = PathBuf::from(source_path);

        if !source_path.exists() {
            return Err(anyhow!(
                "Config file does not exist: {}",
                source_path.display()
            ));
        }

        // Read and validate the config
        let contents =
            fs::read_to_string(&source_path).context("Failed to read source config file")?;
        let config: Config =
            serde_yaml::from_str(&contents).context("Failed to parse source config file")?;

        // Generate a unique filename
        let base_name = source_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("imported");

        let mut target_name = base_name.to_string();
        let mut counter = 1;

        // Find a unique name if there's a conflict
        while self.imported_configs.contains_key(&target_name) {
            target_name = format!("{base_name}_{counter}");
            counter += 1;
        }

        // Copy the file to configs directory
        let target_path = self.configs_dir.join(format!("{target_name}.yaml"));
        fs::copy(&source_path, &target_path).context("Failed to copy config file")?;

        // Add to imported configs
        self.imported_configs.insert(target_name.clone(), config);

        Ok(target_name)
    }

    pub async fn import_config_from_github(&mut self, repo: &str) -> Result<String> {
        // Parse repository format (owner/repo)
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid GitHub repository format. Expected: owner/repo (e.g., username/my-configs)"
            ));
        }

        let (owner, repo_name) = (parts[0], parts[1]);

        // Validate repository format
        if owner.is_empty() || repo_name.is_empty() {
            return Err(anyhow!(
                "Invalid repository format. Both owner and repository name must be non-empty"
            ));
        }

        // First try public API access
        match self.import_from_public_github(owner, repo_name).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Check if it might be a private repository or access issue
                let error_msg = e.to_string();
                if error_msg.contains("not found") || error_msg.contains("404") {
                    println!("⚠ Repository not accessible via public API, trying Git clone (for private repositories)...");

                    // Fallback to git clone for private repositories
                    self.import_from_private_github(owner, repo_name).await
                } else {
                    // Re-throw other errors (network issues, etc.)
                    Err(e)
                }
            }
        }
    }

    async fn import_from_public_github(&mut self, owner: &str, repo_name: &str) -> Result<String> {
        let client = reqwest::Client::new();
        client
            .get("https://api.github.com/user")
            .header("User-Agent", "doo-cli/0.1.0")
            .send()
            .await
            .map_err(|_| {
                anyhow!("Failed to connect to GitHub. Please check your internet connection")
            })?;

        // First, verify the repository exists
        let repo_url = format!("https://api.github.com/repos/{owner}/{repo_name}");
        let repo_response = client
            .get(&repo_url)
            .header("User-Agent", "doo-cli/0.1.0")
            .send()
            .await
            .map_err(|_| anyhow!("Failed to connect to GitHub API"))?;

        if repo_response.status() == 404 {
            return Err(anyhow!(
                "Repository '{}/{}' not found. Please check:\n  • Repository exists\n  • Repository is public\n  • Repository name is spelled correctly", 
                owner, repo_name
            ));
        } else if !repo_response.status().is_success() {
            return Err(anyhow!(
                "Failed to access repository '{}/{}': HTTP {}",
                owner,
                repo_name,
                repo_response.status()
            ));
        }

        // Look for doo.yaml or doo.yml in the repository root
        let config_files = ["doo.yaml", "doo.yml"];
        let mut config_content = None;

        for config_file in config_files {
            let file_url =
                format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{config_file}");

            let response = client
                .get(&file_url)
                .header("User-Agent", "doo-cli/0.1.0")
                .send()
                .await
                .map_err(|_| anyhow!("Failed to fetch config file from GitHub"))?;

            if response.status().is_success() {
                let github_content: GitHubContent = response
                    .json()
                    .await
                    .map_err(|_| anyhow!("Failed to parse GitHub API response"))?;

                if github_content.encoding == "base64" {
                    let decoded_content = base64::decode(github_content.content.replace('\n', ""))
                        .map_err(|_| anyhow!("Failed to decode base64 content from GitHub"))?;

                    config_content = Some(
                        String::from_utf8(decoded_content)
                            .map_err(|_| anyhow!("Config file contains invalid UTF-8"))?,
                    );
                    break;
                }
            }
        }

        let config_content = config_content.ok_or_else(|| {
            anyhow!(
                "No doo configuration file found in repository '{}/{}'. \
                Expected 'doo.yaml' or 'doo.yml' in the repository root.\n\
                \nTo create a shareable config repository:\n\
                1. Create a new GitHub repository\n\
                2. Add a 'doo.yaml' file with your commands:\n\
                   ```yaml\n\
                   commands:\n\
                     command-name: \"command template with #1 #2\"\n\
                   ```\n\
                3. Make the repository public\n\
                4. Share the repository with: doo import owner/repo-name",
                owner,
                repo_name
            )
        })?;

        self.save_imported_config(
            repo_name,
            &config_content,
            &format!("{owner}/{repo_name}"),
            ImportType::Public,
        )
    }

    async fn import_from_private_github(&mut self, owner: &str, repo_name: &str) -> Result<String> {
        // Check if git is available
        let git_check = Command::new("git").arg("--version").output();

        if git_check.is_err() {
            return Err(anyhow!(
                "Git command not found. To import from private repositories, you need:\n\
                • Git installed and available in PATH\n\
                • Proper authentication set up (SSH keys or Git credentials)\n\
                \nAlternatively, make the repository public to use API access."
            ));
        }

        println!("🔐 Attempting to clone private repository (using your Git credentials)...");

        // Create a temporary directory
        let temp_dir =
            TempDir::new().context("Failed to create temporary directory for git clone")?;

        let temp_path = temp_dir.path();
        let repo_path = temp_path.join("repo");

        // Try different Git URL formats
        let git_urls = [
            format!("git@github.com:{owner}/{repo_name}.git"), // SSH
            format!("https://github.com/{owner}/{repo_name}.git"), // HTTPS
        ];

        let mut clone_success = false;
        let mut last_error = String::new();

        for git_url in &git_urls {
            println!("📥 Trying to clone: {git_url}");

            let clone_result = Command::new("git")
                .arg("clone")
                .arg("--depth=1") // Shallow clone for efficiency
                .arg("--quiet") // Reduce noise
                .arg(git_url)
                .arg(&repo_path)
                .output();

            match clone_result {
                Ok(output) => {
                    if output.status.success() {
                        clone_success = true;
                        println!("✅ Successfully cloned repository");
                        break;
                    } else {
                        last_error = String::from_utf8_lossy(&output.stderr).to_string();
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        if !clone_success {
            return Err(anyhow!(
                "Failed to clone private repository '{}/{}'. Please ensure:\n\
                • You have access to the repository\n\
                • Your Git authentication is set up correctly:\n\
                  - SSH: Add your SSH key to GitHub (recommended)\n\
                  - HTTPS: Configure Git credentials or use a personal access token\n\
                • Repository exists and name is spelled correctly\n\
                \nLast error: {}",
                owner,
                repo_name,
                last_error
            ));
        }

        // Look for config files in the cloned repository
        let config_files = ["doo.yaml", "doo.yml"];
        let mut config_content = None;

        for config_file in &config_files {
            let config_path = repo_path.join(config_file);
            if config_path.exists() {
                config_content = Some(fs::read_to_string(&config_path).with_context(|| {
                    format!("Failed to read {config_file} from cloned repository")
                })?);
                println!("📄 Found configuration file: {config_file}");
                break;
            }
        }

        let config_content = config_content.ok_or_else(|| {
            anyhow!(
                "No doo configuration file found in repository '{}/{}'. \
                Expected 'doo.yaml' or 'doo.yml' in the repository root.\n\
                \nTo create a shareable config repository:\n\
                1. Create a new GitHub repository (public or private)\n\
                2. Add a 'doo.yaml' file with your commands:\n\
                   ```yaml\n\
                   commands:\n\
                     command-name: \"command template with #1 #2\"\n\
                   ```\n\
                3. Commit and push the file\n\
                4. Share the repository with: doo import owner/repo-name",
                owner,
                repo_name
            )
        })?;

        // The temporary directory will be automatically cleaned up when temp_dir goes out of scope
        println!("🧹 Cleaning up temporary files...");

        self.save_imported_config(
            repo_name,
            &config_content,
            &format!("{owner}/{repo_name}"),
            ImportType::Private,
        )
    }

    fn save_imported_config(
        &mut self,
        repo_name: &str,
        config_content: &str,
        repo: &str,
        import_type: ImportType,
    ) -> Result<String> {
        // Parse and validate the config
        let mut config: Config = serde_yaml::from_str(config_content).context(
            "Failed to parse config file. Please ensure it follows the correct YAML format",
        )?;

        if config.commands.is_empty() {
            return Err(anyhow!(
                "Config file found but contains no commands. Please add commands to the 'commands' section"
            ));
        }

        // Add origin information
        config.origin = Some(ConfigOrigin {
            repo: repo.to_string(),
            import_type,
        });

        // Generate a unique filename based on the repository name
        let mut target_name = repo_name.to_string();
        let mut counter = 1;

        // Find a unique name if there's a conflict
        while self.imported_configs.contains_key(&target_name) {
            target_name = format!("{repo_name}_{counter}");
            counter += 1;
        }

        // Save the config file to configs directory with origin information
        let config_with_origin = serde_yaml::to_string(&config)
            .context("Failed to serialize config with origin information")?;
        let target_path = self.configs_dir.join(format!("{target_name}.yaml"));
        fs::write(&target_path, config_with_origin)
            .context("Failed to save imported config file")?;

        // Add to imported configs
        self.imported_configs.insert(target_name.clone(), config);

        Ok(target_name)
    }

    pub async fn import_repo_configs(&mut self, repo: &str) -> Result<Vec<String>> {
        // Parse repository format (owner/repo)
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid GitHub repository format. Expected: owner/repo (e.g., username/my-configs)"
            ));
        }

        let (owner, repo_name) = (parts[0], parts[1]);

        // Validate repository format
        if owner.is_empty() || repo_name.is_empty() {
            return Err(anyhow!(
                "Invalid repository format. Both owner and repository name must be non-empty"
            ));
        }

        println!("📦 Importing all YAML configs from repository '{repo}'...");

        // Check if git is available
        let git_check = Command::new("git").arg("--version").output();
        if git_check.is_err() {
            return Err(anyhow!(
                "Git command not found. To import repository configs, you need:\n\
                • Git installed and available in PATH\n\
                • Proper authentication set up (SSH keys or Git credentials)"
            ));
        }

        // Create repository-specific directory in configs
        let repo_dir = self.configs_dir.join(format!("{owner}-{repo_name}"));

        // If directory already exists, remove it first
        if repo_dir.exists() {
            println!("📁 Repository already imported, updating...");
            fs::remove_dir_all(&repo_dir)
                .context("Failed to remove existing repository directory")?;
        }

        fs::create_dir_all(&repo_dir).context("Failed to create repository directory")?;

        println!("🔐 Cloning repository (using your Git credentials)...");

        // Try different Git URL formats
        let git_urls = [
            format!("git@github.com:{repo}.git"),     // SSH
            format!("https://github.com/{repo}.git"), // HTTPS
        ];

        let mut clone_success = false;
        let mut last_error = String::new();

        for git_url in &git_urls {
            println!("📥 Trying to clone: {git_url}");

            let clone_result = Command::new("git")
                .arg("clone")
                .arg("--depth=1") // Shallow clone for efficiency
                .arg("--quiet") // Reduce noise
                .arg(git_url)
                .arg(&repo_dir)
                .output();

            match clone_result {
                Ok(output) => {
                    if output.status.success() {
                        clone_success = true;
                        println!("✅ Successfully cloned repository");
                        break;
                    } else {
                        last_error = String::from_utf8_lossy(&output.stderr).to_string();
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        if !clone_success {
            // Clean up failed directory
            let _ = fs::remove_dir_all(&repo_dir);
            return Err(anyhow!(
                "Failed to clone repository '{}'. Please ensure:\n\
                • You have access to the repository\n\
                • Your Git authentication is set up correctly:\n\
                  - SSH: Add your SSH key to GitHub (recommended)\n\
                  - HTTPS: Configure Git credentials or use a personal access token\n\
                • Repository exists and name is spelled correctly\n\
                \nLast error: {}",
                repo,
                last_error
            ));
        }

        // Keep .git directory for syncing functionality
        println!("📁 Preserving git structure for future sync operations");

        // Find all YAML files in the repository root
        let mut imported_configs = Vec::new();
        let yaml_extensions = ["yaml", "yml"];

        for entry in fs::read_dir(&repo_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if yaml_extensions.contains(&ext_str) {
                            match self.process_repo_yaml_file(&path, repo, &repo_dir) {
                                Ok(config_name) => {
                                    imported_configs.push(config_name);
                                    println!(
                                        "✅ Imported config: {}",
                                        path.file_name().unwrap().to_string_lossy()
                                    );
                                }
                                Err(e) => {
                                    println!(
                                        "⚠ Skipped {}: {}",
                                        path.file_name().unwrap().to_string_lossy(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if imported_configs.is_empty() {
            // Clean up empty directory
            let _ = fs::remove_dir_all(&repo_dir);
            return Err(anyhow!(
                "No valid YAML configuration files found in repository '{}' root directory.\n\
                \nTo create a multi-config repository:\n\
                1. Create YAML files in the repository root (e.g., network.yaml, docker.yaml)\n\
                2. Each file should follow the doo config format:\n\
                   ```yaml\n\
                   # yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json\n\
                   commands:\n\
                     command-name: \"command template with #1 #2\"\n\
                   ```\n\
                3. Commit and push the files\n\
                4. Import with: doo import-repo owner/repo-name",
                repo
            ));
        }

        println!(
            "🎉 Successfully imported {} config file(s) from repository '{}'",
            imported_configs.len(),
            repo
        );
        Ok(imported_configs)
    }

    fn process_repo_yaml_file(
        &mut self,
        file_path: &PathBuf,
        repo: &str,
        _repo_dir: &Path,
    ) -> Result<String> {
        let contents = fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {file_path:?}"))?;

        // Try to parse as a doo config
        let mut config: Config = serde_yaml::from_str(&contents)
            .with_context(|| format!("Failed to parse YAML file: {file_path:?}"))?;

        // Check if it has commands (required for doo configs)
        if config.commands.is_empty() {
            return Err(anyhow!(
                "File contains no commands section or commands are empty"
            ));
        }

        // Add origin information
        config.origin = Some(ConfigOrigin {
            repo: repo.to_string(),
            import_type: ImportType::Private, // Repository imports are treated as private
        });

        // Generate config name from file name and repository
        let file_stem = file_path
            .file_stem()
            .and_then(|name| name.to_str())
            .context("Invalid file name")?;

        let repo_parts: Vec<&str> = repo.split('/').collect();
        let repo_name = repo_parts.get(1).unwrap_or(&repo_parts[0]);
        let config_name = format!("{repo_name}_{file_stem}");

        // Check for conflicts and generate unique name
        let mut unique_name = config_name.clone();
        let mut counter = 1;
        while self.imported_configs.contains_key(&unique_name) {
            unique_name = format!("{config_name}_{counter}");
            counter += 1;
        }

        // Save config with schema reference preserved
        let config_with_schema = if contents.trim_start().starts_with("# yaml-language-server:") {
            // Preserve the schema reference
            let lines: Vec<&str> = contents.lines().collect();
            let mut config_lines = Vec::new();

            // Add schema line if present
            if let Some(first_line) = lines.first() {
                if first_line.starts_with("# yaml-language-server:") {
                    config_lines.push(first_line.to_string());
                    config_lines.push("".to_string()); // Empty line
                }
            }

            // Add the config with origin
            let config_yaml =
                serde_yaml::to_string(&config).context("Failed to serialize config")?;
            config_lines.push(config_yaml);
            config_lines.join("\n")
        } else {
            // Add schema reference and config
            format!(
                "# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json\n\n{}",
                serde_yaml::to_string(&config).context("Failed to serialize config")?
            )
        };

        // Keep the file in the repository directory with its original name
        fs::write(file_path, config_with_schema)
            .context("Failed to update config file with origin information")?;

        // Add to imported configs with the unique name as key but store repo path info
        self.imported_configs.insert(unique_name.clone(), config);

        Ok(unique_name)
    }

    pub async fn sync_all_configs(&mut self) -> Result<()> {
        // Collect configs that have origins
        let syncable_configs: Vec<(String, ConfigOrigin)> = self
            .imported_configs
            .iter()
            .filter_map(|(name, config)| {
                config
                    .origin
                    .as_ref()
                    .map(|origin| (name.clone(), origin.clone()))
            })
            .collect();

        // Also collect GitHub repository directories
        let mut github_repos = Vec::new();
        if self.configs_dir.exists() {
            for entry in fs::read_dir(&self.configs_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                    // Check if this looks like a GitHub repo directory (contains owner-repo format)
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if dir_name.contains('-') {
                            // Check if there's a .git directory or if we can determine it's a GitHub repo
                            let git_dir = path.join(".git");
                            if git_dir.exists() || self.looks_like_github_repo(&path) {
                                github_repos.push((dir_name.to_string(), path.clone()));
                            }
                        }
                    }
                }
            }
        }

        if syncable_configs.is_empty() && github_repos.is_empty() {
            println!("📦 No imported configs with remote origins found. Nothing to sync.");
            return Ok(());
        }

        println!("\n🔄 Config Sync Overview");
        println!("═══════════════════════");

        if !syncable_configs.is_empty() {
            println!(
                "Found {} individual config(s) with remote origins:",
                syncable_configs.len()
            );

            for (name, origin) in &syncable_configs {
                let sync_type = match origin.import_type {
                    ImportType::Public => "📖 Public",
                    ImportType::Private => "🔐 Private",
                };
                println!("  • {name} → {sync_type} ({}) ", origin.repo);
            }
        }

        if !github_repos.is_empty() {
            println!(
                "Found {} GitHub repository director(ies):",
                github_repos.len()
            );
            for (repo_name, _) in &github_repos {
                println!("  • {repo_name} → 🔐 Git Repository");
            }
        }

        println!("\n⚠️  WARNING: This will overwrite all local changes in imported configs!");
        println!("   Local modifications will be lost and replaced with remote content.");

        let confirmed = Confirm::new()
            .with_prompt("Do you want to continue with the sync?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("❌ Sync cancelled by user.");
            return Ok(());
        }

        println!("\n🚀 Starting sync process...\n");

        let mut sync_results = Vec::new();

        // Sync individual configs with origins
        for (config_name, origin) in syncable_configs {
            print!("🔄 Syncing {config_name} from {}... ", origin.repo);

            match self.sync_single_config(&config_name, &origin).await {
                Ok(()) => {
                    println!("✅ Success");
                    sync_results.push((config_name, true, None));
                }
                Err(e) => {
                    println!("❌ Failed");
                    println!("   Error: {e}");
                    sync_results.push((config_name, false, Some(e.to_string())));
                }
            }
        }

        // Sync GitHub repository directories using git commands
        for (repo_name, repo_path) in github_repos {
            print!("🔄 Syncing repository {repo_name}... ");

            match self.sync_github_repository(&repo_path).await {
                Ok(()) => {
                    println!("✅ Success");
                    sync_results.push((repo_name.clone(), true, None));

                    // Reload configs from the updated repository
                    if let Err(e) = self.reload_repo_configs(&repo_path, &repo_name) {
                        println!("⚠️  Warning: Failed to reload configs from {repo_name}: {e}");
                    }
                }
                Err(e) => {
                    println!("❌ Failed");
                    println!("   Error: {e}");
                    sync_results.push((repo_name, false, Some(e.to_string())));
                }
            }
        }

        // Print summary
        println!("\n📊 Sync Summary");
        println!("═══════════════");

        let successful = sync_results
            .iter()
            .filter(|(_, success, _)| *success)
            .count();
        let failed = sync_results.len() - successful;

        println!("✅ Successful: {successful}");
        if failed > 0 {
            println!("❌ Failed: {failed}");
            println!("\nFailed configs:");
            for (name, success, error) in sync_results {
                if !success {
                    println!(
                        "  • {name}: {}",
                        error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                }
            }
        }

        if successful > 0 {
            println!("\n🎉 Sync completed! {successful} config(s) updated successfully.");
        }

        Ok(())
    }

    async fn sync_single_config(&mut self, config_name: &str, origin: &ConfigOrigin) -> Result<()> {
        let parts: Vec<&str> = origin.repo.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow!(
                "Invalid repository format in origin: {}",
                origin.repo
            ));
        }

        let (owner, repo_name) = (parts[0], parts[1]);

        // Fetch the latest config content based on the import type
        let config_content = match origin.import_type {
            ImportType::Public => self.fetch_public_config_content(owner, repo_name).await?,
            ImportType::Private => self.fetch_private_config_content(owner, repo_name).await?,
        };

        // Parse and validate the config
        let mut config: Config = serde_yaml::from_str(&config_content)
            .context("Failed to parse updated config file from remote")?;

        if config.commands.is_empty() {
            return Err(anyhow!("Updated config file contains no commands"));
        }

        // Preserve the origin information
        config.origin = Some(origin.clone());

        // Update the config file on disk
        let config_with_origin =
            serde_yaml::to_string(&config).context("Failed to serialize updated config")?;
        let target_path = self.configs_dir.join(format!("{config_name}.yaml"));
        fs::write(&target_path, config_with_origin)
            .context("Failed to save updated config file")?;

        // Update in-memory config
        self.imported_configs
            .insert(config_name.to_string(), config);

        Ok(())
    }

    /// Check if a directory looks like a GitHub repository directory
    fn looks_like_github_repo(&self, path: &Path) -> bool {
        // Check if directory contains YAML files (typical for imported repos)
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let file_path = entry.path();
                if file_path.is_file() {
                    if let Some(extension) = file_path.extension() {
                        if extension == "yaml" || extension == "yml" {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Sync a GitHub repository directory using git commands
    async fn sync_github_repository(&self, repo_path: &Path) -> Result<()> {
        // Check if git is available
        let git_check = Command::new("git").arg("--version").output();
        if git_check.is_err() {
            return Err(anyhow!(
                "Git command not found. Repository sync requires Git to be installed and available in PATH"
            ));
        }

        // Check if this is a git repository
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow!(
                "Directory is not a git repository. Cannot sync without git history."
            ));
        }

        // Change to the repository directory and run git commands
        // First, fetch all remote changes
        let fetch_result = Command::new("git")
            .current_dir(repo_path)
            .arg("fetch")
            .arg("--all")
            .arg("--prune")
            .output();

        match fetch_result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(anyhow!("Failed to fetch remote changes: {}", stderr.trim()));
                }
            }
            Err(e) => {
                return Err(anyhow!("Failed to execute git fetch: {}", e));
            }
        }

        // Force reset to origin/main (or master) - this will overwrite local changes
        let branches = ["origin/main", "origin/master"];
        let mut reset_success = false;
        let mut last_error = String::new();

        for branch in &branches {
            let reset_result = Command::new("git")
                .current_dir(repo_path)
                .arg("reset")
                .arg("--hard")
                .arg(branch)
                .output();

            match reset_result {
                Ok(output) => {
                    if output.status.success() {
                        reset_success = true;
                        break;
                    } else {
                        last_error = String::from_utf8_lossy(&output.stderr).to_string();
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                }
            }
        }

        if !reset_success {
            return Err(anyhow!(
                "Failed to reset repository to remote state. Last error: {}",
                last_error.trim()
            ));
        }

        // Clean up any untracked files
        let clean_result = Command::new("git")
            .current_dir(repo_path)
            .arg("clean")
            .arg("-fd") // Force remove untracked files and directories
            .output();

        if let Err(e) = clean_result {
            // Log warning but don't fail the sync for clean errors
            eprintln!("Warning: Failed to clean untracked files: {}", e);
        }

        Ok(())
    }

    /// Reload configs from an updated repository directory
    fn reload_repo_configs(&mut self, repo_path: &Path, repo_name: &str) -> Result<()> {
        // Remove old configs from this repository
        let keys_to_remove: Vec<String> = self
            .imported_configs
            .keys()
            .filter(|key| key.starts_with(&format!("{}_", repo_name)))
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.imported_configs.remove(&key);
        }

        // Reload configs from the repository directory
        let yaml_extensions = ["yaml", "yml"];
        for entry in fs::read_dir(repo_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if yaml_extensions.contains(&ext_str) {
                            // Try to load as a doo config
                            if let Ok(contents) = fs::read_to_string(&path) {
                                if let Ok(config) = serde_yaml::from_str::<Config>(&contents) {
                                    if !config.commands.is_empty() {
                                        let file_stem = path
                                            .file_stem()
                                            .and_then(|name| name.to_str())
                                            .unwrap_or("config");
                                        let config_name = format!("{repo_name}_{file_stem}");
                                        self.imported_configs.insert(config_name, config);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn fetch_public_config_content(&self, owner: &str, repo_name: &str) -> Result<String> {
        let client = reqwest::Client::new();

        // Look for doo.yaml or doo.yml in the repository root
        let config_files = ["doo.yaml", "doo.yml"];

        for config_file in config_files {
            let file_url =
                format!("https://api.github.com/repos/{owner}/{repo_name}/contents/{config_file}");

            let response = client
                .get(&file_url)
                .header("User-Agent", "doo-cli/0.1.0")
                .send()
                .await
                .map_err(|_| anyhow!("Failed to fetch config file from GitHub"))?;

            if response.status().is_success() {
                let github_content: GitHubContent = response
                    .json()
                    .await
                    .map_err(|_| anyhow!("Failed to parse GitHub API response"))?;

                if github_content.encoding == "base64" {
                    let decoded_content = base64::decode(github_content.content.replace('\n', ""))
                        .map_err(|_| anyhow!("Failed to decode base64 content from GitHub"))?;

                    return String::from_utf8(decoded_content)
                        .map_err(|_| anyhow!("Config file contains invalid UTF-8"));
                }
            }
        }

        Err(anyhow!(
            "No doo configuration file found in repository '{owner}/{repo_name}'"
        ))
    }

    async fn fetch_private_config_content(&self, owner: &str, repo_name: &str) -> Result<String> {
        // Create a temporary directory
        let temp_dir =
            TempDir::new().context("Failed to create temporary directory for git clone")?;

        let temp_path = temp_dir.path();
        let repo_path = temp_path.join("repo");

        // Try different Git URL formats
        let git_urls = [
            format!("git@github.com:{owner}/{repo_name}.git"), // SSH
            format!("https://github.com/{owner}/{repo_name}.git"), // HTTPS
        ];

        let mut clone_success = false;

        for git_url in &git_urls {
            let clone_result = Command::new("git")
                .arg("clone")
                .arg("--depth=1") // Shallow clone for efficiency
                .arg("--quiet") // Reduce noise
                .arg(git_url)
                .arg(&repo_path)
                .output();

            match clone_result {
                Ok(output) => {
                    if output.status.success() {
                        clone_success = true;
                        break;
                    }
                }
                Err(_) => continue,
            }
        }

        if !clone_success {
            return Err(anyhow!(
                "Failed to clone private repository '{owner}/{repo_name}' during sync"
            ));
        }

        // Look for config files in the cloned repository
        let config_files = ["doo.yaml", "doo.yml"];

        for config_file in &config_files {
            let config_path = repo_path.join(config_file);
            if config_path.exists() {
                return fs::read_to_string(&config_path).with_context(|| {
                    format!("Failed to read {config_file} from cloned repository")
                });
            }
        }

        Err(anyhow!(
            "No doo configuration file found in repository '{owner}/{repo_name}'"
        ))
    }

    pub fn get_command(&self, name: &str) -> Result<Option<String>> {
        // First check main config
        if let Some(entry) = self.main_config.commands.get(name) {
            return Ok(Some(entry.command_str().to_string()));
        }

        for config in self.imported_configs.values() {
            if let Some(entry) = config.commands.get(name) {
                return Ok(Some(entry.command_str().to_string()));
            }
        }
        Ok(None)
    }

    pub fn get_command_with_source(&self, name: &str) -> Result<Option<CommandSource>> {
        // First check main config
        if let Some(entry) = self.main_config.commands.get(name) {
            return Ok(Some(CommandSource {
                name: name.to_string(),
                command: entry.command_str().to_string(),
                description: entry.description().map(|s| s.to_string()),
                source_file: "main".to_string(),
            }));
        }
        for (config_name, config) in &self.imported_configs {
            if let Some(entry) = config.commands.get(name) {
                return Ok(Some(CommandSource {
                    name: name.to_string(),
                    command: entry.command_str().to_string(),
                    description: entry.description().map(|s| s.to_string()),
                    source_file: config_name.clone(),
                }));
            }
        }
        Ok(None)
    }

    pub fn get_command_conflicts(&self, name: &str) -> Vec<CommandSource> {
        let mut conflicts = Vec::new();

        // Check main config
        if let Some(entry) = self.main_config.commands.get(name) {
            conflicts.push(CommandSource {
                name: name.to_string(),
                command: entry.command_str().to_string(),
                description: entry.description().map(|s| s.to_string()),
                source_file: "main".to_string(),
            });
        }
        for (config_name, config) in &self.imported_configs {
            if let Some(entry) = config.commands.get(name) {
                conflicts.push(CommandSource {
                    name: name.to_string(),
                    command: entry.command_str().to_string(),
                    description: entry.description().map(|s| s.to_string()),
                    source_file: config_name.clone(),
                });
            }
        }
        conflicts
    }

    pub fn resolve_command_conflict(
        &self,
        name: &str,
        chosen_source: &str,
    ) -> Result<Option<String>> {
        if chosen_source == "main" {
            return Ok(self
                .main_config
                .commands
                .get(name)
                .map(|e| e.command_str().to_string()));
        }
        if let Some(config) = self.imported_configs.get(chosen_source) {
            return Ok(config
                .commands
                .get(name)
                .map(|e| e.command_str().to_string()));
        }
        Err(anyhow!("Invalid source file: {}", chosen_source))
    }

    pub fn add_command(&mut self, name: &str, command: &str) -> Result<()> {
        self.main_config
            .commands
            .insert(name.to_string(), CommandEntry::Simple(command.to_string()));
        self.save_main_config()
    }

    pub fn remove_command(&mut self, name: &str) -> Result<bool> {
        let removed = self.main_config.commands.remove(name).is_some();
        if removed {
            self.save_main_config()?;
        }
        Ok(removed)
    }

    pub fn list_commands(&self) -> HashMap<String, String> {
        let mut all_commands = HashMap::new();
        for (name, entry) in &self.main_config.commands {
            all_commands.insert(name.clone(), entry.command_str().to_string());
        }
        for config in self.imported_configs.values() {
            for (name, entry) in &config.commands {
                all_commands.insert(name.clone(), entry.command_str().to_string());
            }
        }
        all_commands
    }

    pub fn search_commands(&self, query: &str) -> Vec<CommandSearchResult> {
        let q = query.to_lowercase();
        let mut results = Vec::new();

        // Iterate through merged view (imported override main). We'll prefer imported variant already handled by iteration order (main then imported overwrite) but for description we just display whichever ends up.
        let mut merged: HashMap<String, &CommandEntry> = HashMap::new();
        for (name, entry) in &self.main_config.commands {
            merged.insert(name.clone(), entry);
        }
        for config in self.imported_configs.values() {
            for (name, entry) in &config.commands {
                merged.insert(name.clone(), entry); // override
            }
        }

        for (name, entry) in merged {
            let cmd = entry.command_str();
            let desc = entry.description();
            if q.is_empty()
                || name.to_lowercase().contains(&q)
                || cmd.to_lowercase().contains(&q)
                || desc.map(|d| d.to_lowercase().contains(&q)).unwrap_or(false)
            {
                results.push(CommandSearchResult {
                    name,
                    command: cmd.to_string(),
                    description: desc.map(|s| s.to_string()),
                });
            }
        }
        // Sort by name for stable display
        results.sort_by(|a, b| a.name.cmp(&b.name));
        results
    }

    fn save_main_config(&self) -> Result<()> {
        let config_file = self.config_dir.join("config.yaml");
        let yaml_content =
            serde_yaml::to_string(&self.main_config).context("Failed to serialize config")?;
        fs::write(&config_file, yaml_content).context("Failed to write config file")?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn new_with_dir(config_dir: PathBuf) -> Result<Self> {
        // Create config directory if it doesn't exist
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        // Create configs subdirectory for imported configs
        let configs_dir = config_dir.join("configs");
        fs::create_dir_all(&configs_dir).context("Failed to create configs directory")?;

        let config_file = config_dir.join("config.yaml");
        let main_config = if config_file.exists() {
            let contents =
                fs::read_to_string(&config_file).context("Failed to read config file")?;
            serde_yaml::from_str(&contents).context("Failed to parse config file")?
        } else {
            Config::default()
        };

        // Load all imported configs from files and repository directories
        let mut imported_configs = HashMap::new();
        if configs_dir.exists() {
            // Load configs from files in configs directory
            for entry in fs::read_dir(&configs_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file()
                    && path
                        .extension()
                        .is_some_and(|ext| ext == "yaml" || ext == "yml")
                {
                    let file_name = path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .context("Invalid config file name")?
                        .to_string();

                    let contents = fs::read_to_string(&path)
                        .with_context(|| format!("Failed to read config file: {path:?}"))?;
                    let config: Config = serde_yaml::from_str(&contents)
                        .with_context(|| format!("Failed to parse config file: {path:?}"))?;

                    imported_configs.insert(file_name, config);
                }
            }

            // Load configs from repository directories
            for entry in fs::read_dir(&configs_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() && !path.file_name().unwrap().to_str().unwrap().starts_with('.') {
                    // This is a repository directory, scan for YAML files
                    for repo_entry in fs::read_dir(&path)? {
                        let repo_entry = repo_entry?;
                        let repo_file_path = repo_entry.path();

                        if repo_file_path.is_file()
                            && repo_file_path
                                .extension()
                                .is_some_and(|ext| ext == "yaml" || ext == "yml")
                        {
                            let repo_name = path.file_name().unwrap().to_str().unwrap();
                            let file_stem = repo_file_path
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("config");

                            // Create unique config name: repo_filename
                            let config_name = format!("{repo_name}_{file_stem}");

                            let contents =
                                fs::read_to_string(&repo_file_path).with_context(|| {
                                    format!("Failed to read repo config file: {repo_file_path:?}")
                                })?;

                            if let Ok(config) = serde_yaml::from_str::<Config>(&contents) {
                                // Only add if it's a valid doo config with commands
                                if !config.commands.is_empty() {
                                    imported_configs.insert(config_name, config);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            config_dir,
            configs_dir,
            main_config,
            imported_configs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config").join("doo");

        let config_manager = ConfigManager::new_with_dir(config_dir);
        assert!(config_manager.is_ok());
    }

    #[test]
    fn test_command_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_dir = temp_dir.path().join(".config").join("doo");

        let mut config_manager = ConfigManager::new_with_dir(config_dir).unwrap();

        // Test adding command
        config_manager.add_command("test", "echo hello").unwrap();
        assert_eq!(
            config_manager.get_command("test").unwrap(),
            Some("echo hello".to_string())
        );

        // Test removing command
        assert!(config_manager.remove_command("test").unwrap());
        assert_eq!(config_manager.get_command("test").unwrap(), None);
    }
}
