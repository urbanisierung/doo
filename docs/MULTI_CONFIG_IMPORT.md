# Multi-Config Repository Import Test

This document demonstrates the new multi-config repository import feature.

## Overview

The `doo import-repo` command allows you to import all YAML configuration files from a GitHub repository in one operation. This is useful for:

- Organizing commands by topic (docker.yaml, kubernetes.yaml, network.yaml)
- Sharing comprehensive command libraries
- Maintaining separate configs for different tools/services

## Usage

```bash
# Import all YAML files from a repository
doo import-repo username/my-configs

# This will:
# 1. Clone the repository to ~/.config/doo/configs/username-my-configs/
# 2. Scan all .yaml and .yml files in the repository root
# 3. Import each file as a separate config
# 4. Apply automatic naming (repo_filename format)
# 5. Add schema references to all files
```

## Repository Structure

A multi-config repository should look like:

```
my-configs/
├── docker.yaml      # Docker-related commands
├── kubernetes.yaml  # K8s-related commands
├── network.yaml     # Network troubleshooting
├── database.yaml    # Database operations
└── README.md        # Documentation (ignored)
```

Each YAML file should follow the doo config format:

```yaml
# yaml-language-server: $schema=https://bucket.u11g.com/doo-config.schema.json

commands:
  command-name: "command template with #1 #2"
  another-cmd:
    command: "another command template"
    description: "Optional description shown in interactive menu and searchable"
```

## Benefits

1. **Topic Organization**: Separate configs by functionality
2. **Easy Updates**: `doo sync` updates all imported configs
3. **Conflict Resolution**: Automatic unique naming prevents conflicts
4. **Schema Validation**: All files get schema references automatically
5. **Git Integration**: Works with both public and private repositories

## Implementation

The feature:

- Uses git clone for reliable repository access
- Supports both SSH and HTTPS authentication
- Validates each YAML file for doo config format
- Preserves or adds schema references
- Creates unique config names to avoid conflicts
- Integrates with existing sync functionality
