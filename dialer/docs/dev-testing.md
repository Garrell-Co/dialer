# Testing Setup

The testing setup uses the FreeSWITCH configuration from `/usr/share/freeswitch/conf/vanilla/` as the base configuration, with an overlay system that automatically merges test-specific modifications from [`dialer/test_conf/`](../test_conf/).

## Configuration Overlay System

The configuration system uses a two-layer approach:

1. **Base Configuration**: `/usr/share/freeswitch/conf/vanilla/` - The default FreeSWITCH vanilla configuration installed by the package
2. **Test Overlay**: `dialer/test_conf/` - Test-specific modifications that override the vanilla configuration

Files in `dialer/test_conf/` with the same relative path as files in the vanilla configuration will override the vanilla versions. For example:
- `dialer/test_conf/autoload_configs/event_socket.conf.xml` overrides `/usr/share/freeswitch/conf/vanilla/autoload_configs/event_socket.conf.xml`
- `dialer/test_conf/dialplan/default.xml` overrides `/usr/share/freeswitch/conf/vanilla/dialplan/default.xml`

This approach allows you to track only your test-specific changes in git without managing the entire vanilla configuration directory.

## Configuration Setup in `fs-run`

The [`dialer/tools/fs-run`](../tools/fs-run) script automatically handles the configuration overlay:

1. **Automatic Detection**: When `dialer/test_conf/` exists and contains files, the script creates a temporary merged configuration directory
2. **Merging Process**: 
   - Copies the vanilla configuration as the base
   - Overlays files from `dialer/test_conf/` (files with matching paths override vanilla files)
   - Sets appropriate permissions for the freeswitch user
3. **Fallback**: If `test_conf` doesn't exist or is empty, it uses the vanilla configuration directly
4. **Cleanup**: The temporary merged configuration directory is automatically cleaned up when FreeSWITCH exits

The script runs FreeSWITCH with the following flags:

- `-nonat`: Disables NAT traversal requirements
- `-conf`: Specifies the configuration directory (merged config if test_conf exists, otherwise vanilla)
- `-log`: Specifies the log directory (`/var/log/freeswitch`)
- `-db`: Specifies the database directory (`/var/lib/freeswitch/db`)

This ensures that the development environment automatically uses test-specific modifications when they exist, while falling back to vanilla configuration when they don't.

## Useful `fs_cli` commands

### Modules
- Check if a module is loaded
    - `module_exists <module_name>`
- Load a module
    - `load <module_name>`
- View all loaded modules
    - `show modules`

### Dial plans
- Show all dial plans
    - `show dialplan`