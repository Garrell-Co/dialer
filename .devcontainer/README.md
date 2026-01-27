# Development Container Configuration

This directory contains the development container setup for the dialer project.

## Contents

### Container Definition
- **`Dockerfile`** - Container image definition with all development dependencies
- **`devcontainer.json`** - VS Code devcontainer configuration
- **`setup.sh`** - Post-create script that runs after container is built

### FreeSWITCH Tools
Located in `tools/`:
- **`fs-run`** - Run FreeSWITCH in development mode with test configuration overlay
- **`fs-up`** - Install FreeSWITCH from SignalWire repositories
- **`check-freeswitch.sh`** - Verify FreeSWITCH installation
- **`check-fs-installation.sh`** - Detailed FreeSWITCH installation check
- **`.env`** - Environment variables (create from `.env.example`, contains PAT token)

These tools are automatically symlinked to `~/bin` during container setup, making them available as simple commands (`fs-run`, `fs-up`, `check-freeswitch`) from anywhere in the container.

### FreeSWITCH Configuration
Located in `fs_config/`:
- **`reference_conf/`** - Complete reference copy of vanilla FreeSWITCH configuration (for documentation)
- **`test_conf/`** - Test-specific configuration overlays

The `fs-run` script automatically merges `test_conf/` over the vanilla configuration at `/usr/share/freeswitch/conf/vanilla/` when starting FreeSWITCH. This allows you to track only your test-specific changes without managing the entire configuration directory.

## Setup Process

1. **Container Build**: Docker builds the image using `Dockerfile`
2. **Post-Create**: `setup.sh` runs automatically, creating symlinks for FreeSWITCH tools
3. **FreeSWITCH Install**: User runs `fs-up` to install FreeSWITCH (requires PAT token in `tools/.env`)
4. **Development**: User runs `fs-run` to start FreeSWITCH with test configuration

## Why This Structure?

**Devcontainer-only tools**: The FreeSWITCH scripts (`fs-run`, `fs-up`, etc.) are devcontainer-specific because they:
- Rely on the `freeswitch` user created in the Dockerfile
- Use specific directory permissions set during container build
- Reference paths and configurations specific to the container environment

By placing them in `.devcontainer/`, we make it clear these are infrastructure tools that only work within the development container context.

**Configuration overlay**: The `fs_config/test_conf/` directory allows tracking only the test-specific configuration changes, while the full vanilla configuration is available in `reference_conf/` for reference.

## Related Documentation

- See main [`README.md`](../README.md) for usage instructions
- See [`dialer/docs/dev-testing.md`](../dialer/docs/dev-testing.md) for details on the configuration overlay system

