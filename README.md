# doo - Command Wrapper CLI

A powerful CLI tool that wraps other commands with persistent variables and contexts. Perfect for managing complex commands with environment-specific parameters.

## Features

- 🔧 **Command Wrapping**: Define command templates that can be executed with simple names
- 🔄 **Variable Substitution**: Use persistent variables like `#1`, `#2` that persist across sessions
- 🎯 **Context Management**: Switch between different contexts (e.g., dev, staging, prod) with separate variable sets
- 🔍 **Interactive Menu**: Browse and search through available commands with a terminal-based UI
- 🎨 **Colorful Output**: Nice colors to make the CLI experience pleasant
- 📁 **Cross-Platform**: Works on Linux, macOS, and Windows

## Installation

### Quick Install (Recommended)

**Linux & macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/urbanisierung/doo/main/install.sh | bash
```

**Windows (PowerShell):**

```powershell
iwr -useb https://raw.githubusercontent.com/urbanisierung/doo/main/install.ps1 | iex
```

### From Source

```bash
# Clone the repository
git clone https://github.com/urbanisierung/doo.git
cd doo

# Build and install
cargo build --release
cargo install --path .
```

### Manual Installation

1. Download the latest release from [GitHub Releases](https://github.com/urbanisierung/doo/releases)
2. Extract the binary for your platform
3. Move it to a directory in your PATH (e.g., `/usr/local/bin` on Linux/macOS)

## Quick Start

### 1. Basic Usage

When you first run `doo`, it creates a configuration directory with some example commands:

```bash
# Run without arguments to open the interactive menu
doo

# Execute a predefined command
doo watch

# Execute with positional arguments
doo logs pod-name

# Import additional commands from external files
doo import my-custom-commands.yaml
doo import username/shared-configs
```

### 2. Variable Management

Set persistent variables that work across sessions:

```bash
# Set variable #1 to "production" in current context
doo var #1 production

# Now this command will use "production" as the namespace
doo watch  # Executes: watch kubectl -n production get pods
```

### 3. Context Management

Switch between different environments:

```bash
# Switch to staging context
doo context staging

# Set variables specific to staging
doo var #1 staging-env
doo var #2 my-app

# Variables are now specific to the staging context
doo watch  # Uses staging-specific variables
```

### 4. Config File Management

Import external configuration files to extend your command library:

```bash
# Import a local config file
doo import /path/to/my-commands.yaml
doo import docker-commands.yaml

# Import from GitHub repository (single config file)
doo import username/my-doo-configs
doo import organization/team-commands

# Import all YAML files from a GitHub repository
doo import-repo username/multi-config-repo
doo import-repo organization/team-configs
```

**Single Config Import Requirements:**

- Repository must be public (or private with Git authentication)
- Must contain `doo.yaml` or `doo.yml` in the repository root
- File must follow the standard doo config format

**Repository Import Requirements:**

- Repository must be accessible via Git (public or private with authentication)
- All YAML files in the repository root are imported as separate configs
- Each YAML file must follow the standard doo config format with a `commands` section
- Files are automatically validated and schema references are added

**Repository Import Benefits:**

- Import multiple configuration files organized by topic (e.g., `docker.yaml`, `kubernetes.yaml`, `network.yaml`)
- Automatic conflict resolution with unique naming (`repo_filename` format)
- Repository is cloned to `~/.config/doo/configs/owner-repo/` for easy updates
- All files are kept in sync with their repository source

When importing configs, they are merged with your existing commands. If there are naming conflicts between different config files, `doo` will prompt you to choose which version to use.

### 5. Interactive Menu

Simply run `doo` without arguments to open an interactive menu powered by the mature [dialoguer](https://github.com/console-rs/dialoguer) library:

- **Fuzzy search**: Type to filter through available commands intelligently
- **Arrow navigation**: Use ↑/↓ arrow keys to navigate options
- **Professional UI**: Clean, colorful interface with context display
- **Quick execution**: Press Enter to execute the selected command
- **Easy exit**: Press Esc to cancel and exit

The menu displays your current context and allows real-time filtering of commands as you type.

## Configuration

### Multi-Config File Support

`doo` supports multiple configuration files to organize your commands:

- **Main config**: `~/.config/doo/config.yaml` - Your primary command definitions
- **Imported configs**: `~/.config/doo/configs/*.yaml` - Additional config files imported with `doo import`

All config files are automatically loaded and merged. Commands from imported files are available alongside your main config commands.

### Configuration Structure & Schema

Doo config files follow a standardized YAML structure with JSON Schema validation for better IDE support and error checking.

#### Schema Support

Add this line at the top of your config files to enable schema validation and autocompletion in supported IDEs:

```yaml
# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json
```

#### Basic Structure

```yaml
# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json

commands:
  command-name: "command template"
  another-command: "another template with #1 #2"
```

#### Required Fields

- **`commands`** (object, required): Map of command names to command templates
  - Keys must be valid command names (alphanumeric, underscores, hyphens only)
  - Values must be non-empty strings containing the command template

#### Optional Fields

- **`origin`** (object, optional): Automatically added by `doo import` for tracking remote sources
  - **`repo`** (string): GitHub repository in `owner/repo` format
  - **`import_type`** (enum): Either `"Public"` or `"Private"`

#### Configuration Template

You can use the provided template as a starting point:

```bash
# Copy the template
curl -o my-commands.yaml https://raw.githubusercontent.com/urbanisierung/doo/main/doo-config.template.yaml

# Edit and import
doo import my-commands.yaml
```

#### Validation Rules

1. **Command Names**: Must match pattern `^[a-zA-Z0-9_-]+$`
2. **Command Templates**: Must be non-empty strings
3. **Reserved Names**: Cannot use `var`, `context`, `import`, `sync` as command names
4. **Variable Placeholders**: Use `#1`, `#2` for persistent variables or `$1`, `$2` for direct arguments

#### Example Valid Configuration

```yaml
# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json

commands:
  # Kubernetes commands with persistent variables
  k-watch: "watch kubectl -n #1 get pods"
  k-logs: "kubectl logs -f -n #1 #2"
  k-describe: "kubectl describe pod -n #1 #2"

  # Docker commands with direct arguments
  d-logs: "docker logs -f $1"
  d-exec: "docker exec -it $1 /bin/bash"

  # Mixed approach
  k-mixed: "kubectl -n #1 logs $1 --tail=$2"

  # Simple commands
  git-status: "git status"
  disk-space: "df -h"
```

### Command Templates

Commands can be defined in any config file. The main config file format:

```yaml
commands:
  # Using persistent variables (#1, #2) - can be set with 'doo var'
  watch: "watch kubectl -n #1 get pods"
  logs: "kubectl logs -f -n #1 #2"
  pods: "kubectl get pods -n #1"
  describe: "kubectl describe pod -n #1 #2"
  deploy: "kubectl apply -f deployment.yaml -n #1"
  scale: "kubectl scale deployment #2 --replicas=#3 -n #1"

  # Using direct positional arguments ($1, $2) - always from command line
  quick-pods: "kubectl -n $1 get pods"
  quick-logs: "kubectl logs -f $1 -n $2"
  port-forward: "kubectl port-forward $1 $2:8080"
```

Example imported config file (`docker-commands.yaml`):

```yaml
commands:
  docker-ps: "docker ps"
  docker-logs: "docker logs -f $1" # Direct argument
  docker-exec: "docker exec -it $1 /bin/bash" # Direct argument
  docker-build: "docker build -t #1 ." # Persistent variable
```

### Conflict Resolution

When the same command name exists in multiple config files, `doo` will prompt you to choose which version to use:

```bash
$ doo logs
⚠ Command 'logs' found in multiple config files:
  1) logs (from main): kubectl logs -f -n #1 #2
  2) logs (from docker-commands): docker logs -f #1

Which config file should be used? Enter number (1-2):
```

### Creating Shareable Config Files

You can create config files to share with your team or across different projects:

**For Local Sharing:**

```yaml
# team-kubectl-commands.yaml
commands:
  k-pods: "kubectl get pods -n #1"
  k-logs: "kubectl logs -f -n #1 #2 --tail=100"
  k-exec: "kubectl exec -it -n #1 #2 -- /bin/bash"
  k-describe: "kubectl describe pod -n #1 #2"
  k-port-forward: "kubectl port-forward -n #1 #2 #3:8080"
```

**For GitHub Sharing:**

1. Create a new public GitHub repository (e.g., `my-doo-configs`)
2. Add a `doo.yaml` file in the repository root:

```yaml
commands:
  # Mix of persistent variables and direct arguments
  deploy-staging: "kubectl apply -f deployment.yaml -n staging"
  scale-app: "kubectl scale deployment #1 --replicas=#2 -n #3"
  get-logs: "kubectl logs -f deployment/$1 -n $2" # Direct args
  port-forward: "kubectl port-forward service/$1 $2:8080 -n #3"
  quick-exec: "kubectl exec -it $1 -- $2" # Direct args
```

3. Share with your team: `doo import username/my-doo-configs`

Then team members can import it:

```bash
# Local import
doo import team-kubectl-commands.yaml

# GitHub import
doo import username/my-doo-configs
```

### GitHub Repository Structure

When creating a GitHub repository for sharing doo configs, follow this structure:

```
your-doo-configs/
├── doo.yaml          # Required: Main config file
├── README.md         # Optional: Documentation
└── examples/         # Optional: Usage examples
    └── usage.md
```

**Required `doo.yaml` format with schema support:**

```yaml
# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json

commands:
  # Examples of both placeholder types
  command-name: "command template with #1 #2" # Persistent variables
  direct-args: "command with $1 $2" # Direct arguments
  mixed: "kubectl -n #1 logs $1 $2" # Mixed approach
  simple: "echo Hello $1" # Direct argument
```

**Repository Requirements:**

- Must be public (private repositories are supported via Git clone)
- Must contain `doo.yaml` or `doo.yml` in the root
- Config file must be valid YAML with a `commands` section
- Commands can use `#1`, `#2` for persistent variables or `$1`, `$2` for direct arguments
- Recommended: Include schema reference for IDE support

### Variables

Variables are stored per context in `~/.config/doo/variables/`:

- `default.yaml` - Variables for the default context
- `staging.yaml` - Variables for the staging context
- `production.yaml` - Variables for the production context

Example variable file:

```yaml
vars:
  "#1": "my-namespace"
  "#2": "my-pod"
  "#3": "3"
```

## Command Examples

```bash
# Kubernetes workflow with persistent variables
doo var #1 production           # Set namespace
doo var #2 my-app               # Set app name
doo watch                       # watch kubectl -n production get pods
doo logs my-app-pod-123         # kubectl logs -f -n production my-app-pod-123
doo describe my-app-pod-123     # kubectl describe pod -n production my-app-pod-123

# Direct positional arguments (using $1, $2)
# Command: "kubectl -n $1 get pods"
doo pods staging                # kubectl -n staging get pods
doo pods production             # kubectl -n production get pods

# Command: "kubectl logs -f $1 -n $2"
doo app-logs my-pod staging     # kubectl logs -f my-pod -n staging
doo describe my-app-pod-123     # kubectl describe pod -n production my-app-pod-123

# Import and use Docker commands
doo import docker-commands.yaml # Import local config
doo import username/docker-configs # Import from GitHub (single file)
doo import-repo username/multi-configs # Import all YAML files from repo
doo docker-ps                   # docker ps
doo docker-logs container-name  # docker logs -f container-name

# Different environments
doo context staging
doo var #1 staging-ns
doo watch                       # Now uses staging namespace

doo context production
doo var #1 prod-ns
doo watch                       # Now uses production namespace
```

## Reserved Commands

The following commands are reserved and cannot be overwritten:

- `var` - Manage variables (`doo var #1 value`)
- `context` - Switch contexts (`doo context staging`)
- `import` - Import config files (`doo import config.yaml` or `doo import username/repo`)
- `import-repo` - Import all YAML files from a repository (`doo import-repo username/multi-configs`)
- `sync` - Sync all imported configs with their remote sources (`doo sync`)

## Variable Resolution

doo supports two types of variable placeholders in command templates:

### Placeholder Types

- **`$1`, `$2`, `$3`...**: Direct positional arguments (always replaced by command-line arguments)
- **`#1`, `#2`, `#3`...**: Persistent variables (can be set with `doo var` or use positional arguments as fallback)

### Resolution Order

Variables are resolved in the following order:

1. **Direct positional placeholders** (`$1`, `$2`): Replaced directly with command-line arguments
2. **Persistent variables** (`#1`, `#2`): Variables set with `doo var #1 value`
3. **Positional fallback** (`#1`, `#2`): If not set as persistent variables, use command-line arguments

### Examples

**Using Direct Positional Arguments (`$1`, `$2`):**

```bash
# Command template: "kubectl -n $1 get pods"
doo pods production
# Executes: kubectl -n production get pods

# Command template: "kubectl logs -f $1 -n $2"
doo logs my-pod staging
# Executes: kubectl logs -f my-pod -n staging
```

**Using Persistent Variables (`#1`, `#2`):**

```bash
# Set persistent variable
doo var #1 production

# Command template: "kubectl logs -n #1 #2"
doo logs my-pod
# Executes: kubectl logs -n production my-pod
```

**Mixing Both Types:**

```bash
# Set persistent variable for namespace
doo var #1 production

# Command template: "kubectl -n #1 logs $1 $2"
doo logs my-pod --follow
# Executes: kubectl -n production logs my-pod --follow
```

## Configuration Locations

- **Linux/macOS**: `~/.config/doo/`
- **Windows**: `%APPDATA%\doo\`

The configuration directory contains:

- `config.yaml` - Main command templates
- `configs/` - Directory containing imported config files (\*.yaml) and repository directories
  - `*.yaml` - Individual imported config files
  - `owner-repo/` - Repository directories containing multiple YAML files
- `variables/` - Directory containing variable files per context
- `current_context` - File storing the current active context

## Development

```bash
# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run

# Format code
cargo fmt

# Check with clippy
cargo clippy
```

## Technical Details

This tool is built with modern, mature Rust libraries:

- **CLI Framework**: [clap](https://github.com/clap-rs/clap) - Industry-standard command-line argument parser
- **Interactive UI**: [dialoguer](https://github.com/console-rs/dialoguer) - Professional terminal user interfaces with fuzzy search
- **Configuration**: [serde](https://serde.rs/) + [serde_yaml](https://github.com/dtolnay/serde-yaml) - Robust serialization
- **Cross-platform**: [dirs](https://github.com/dirs-dev/dirs-rs) - Platform-appropriate configuration directories
- **Error Handling**: [anyhow](https://github.com/dtolnay/anyhow) + [thiserror](https://github.com/dtolnay/thiserror) - Ergonomic error management

The interactive menu leverages dialoguer's `FuzzySelect` for a smooth, responsive user experience with intelligent command filtering.

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Installation Scripts

The repository includes cross-platform installation scripts:

- `install.sh` - For Linux and macOS
- `install.ps1` - For Windows PowerShell
- `INSTALL.md` - Detailed installation documentation

These scripts automatically download and install the latest release. See [INSTALL.md](INSTALL.md) for details.

### Configuration Files & Schema

The repository provides schema and template support for config files:

- `doo-config.schema.json` - JSON Schema for YAML validation and IDE support
- `doo-config.template.yaml` - Template file for creating new configurations
- Example config files: `doo.yaml`, `test_config.yaml`, `test-dollar-config.yaml`, `network_config.yaml`

All config files include the schema reference for IDE autocompletion and validation.

## License

This project is licensed under the MIT OR Apache-2.0 license.
