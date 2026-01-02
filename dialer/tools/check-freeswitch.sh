#!/bin/bash
# FreeSWITCH Installation Sanity Check

echo "=== FreeSWITCH Installation Check ==="
echo ""

# Check if FreeSWITCH package is installed
echo "1. Checking if FreeSWITCH package is installed..."
if dpkg -l | grep -q "^ii.*freeswitch"; then
    echo "   ✓ FreeSWITCH package is installed"
    dpkg -l | grep "^ii.*freeswitch" | head -5
else
    echo "   ✗ FreeSWITCH package not found"
fi
echo ""

# Check if FreeSWITCH binary exists and is executable
echo "2. Checking FreeSWITCH binary..."
FS_BINARY=$(which freeswitch 2>/dev/null || echo "")
if [ -n "$FS_BINARY" ]; then
    echo "   ✓ FreeSWITCH binary found at: $FS_BINARY"
    if [ -x "$FS_BINARY" ]; then
        echo "   ✓ Binary is executable"
    else
        echo "   ✗ Binary is not executable"
    fi
else
    echo "   ✗ FreeSWITCH binary not found in PATH"
    echo "   Checking common locations..."
    for path in /usr/bin/freeswitch /usr/local/bin/freeswitch /opt/freeswitch/bin/freeswitch; do
        if [ -f "$path" ]; then
            echo "   Found at: $path"
        fi
    done
fi
echo ""

# Check FreeSWITCH version
echo "3. Checking FreeSWITCH version..."
if command -v freeswitch &> /dev/null; then
    # Try to get version (may require sudo for some operations)
    if sudo -n true 2>/dev/null; then
        echo "   Attempting to get version (requires sudo)..."
        sudo freeswitch -version 2>/dev/null || echo "   Could not get version (may need to run manually)"
    else
        echo "   Run 'sudo freeswitch -version' to check version"
    fi
else
    echo "   ✗ Cannot check version - binary not accessible"
fi
echo ""

# Check if configuration directory exists
echo "4. Checking FreeSWITCH configuration..."
FS_CONF_DIRS=("/etc/freeswitch" "/usr/local/freeswitch/conf" "/opt/freeswitch/conf")
FOUND_CONF=false
for dir in "${FS_CONF_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo "   ✓ Configuration directory found: $dir"
        FOUND_CONF=true
        if [ -r "$dir" ]; then
            echo "   ✓ Configuration directory is readable"
        else
            echo "   ⚠ Configuration directory exists but is not readable"
        fi
        break
    fi
done
if [ "$FOUND_CONF" = false ]; then
    echo "   ✗ Configuration directory not found in common locations"
fi
echo ""

# Check installed modules
echo "5. Checking installed modules..."
if [ -d "/usr/lib/freeswitch/mod" ] || [ -d "/usr/local/freeswitch/mod" ]; then
    MOD_DIR=$(find /usr/lib/freeswitch /usr/local/freeswitch -type d -name "mod" 2>/dev/null | head -1)
    if [ -n "$MOD_DIR" ]; then
        MOD_COUNT=$(find "$MOD_DIR" -name "mod_*.so" 2>/dev/null | wc -l)
        echo "   ✓ Found modules directory: $MOD_DIR"
        echo "   ✓ Found $MOD_COUNT module files"
        echo "   Installed modules:"
        find "$MOD_DIR" -name "mod_*.so" 2>/dev/null | sed 's|.*/mod_|     - mod_|' | sed 's|\.so$||' | head -10
    fi
else
    echo "   ⚠ Modules directory not found"
fi
echo ""

# Summary
echo "=== Summary ==="
if dpkg -l | grep -q "^ii.*freeswitch" && [ -n "$(which freeswitch 2>/dev/null)" ]; then
    echo "✓ FreeSWITCH appears to be installed correctly"
    echo ""
    echo "To test FreeSWITCH, try:"
    echo "  sudo freeswitch -version    # Check version"
    echo "  sudo freeswitch -help       # Show help"
    echo "  sudo systemctl status freeswitch  # Check service status (if systemd is available)"
else
    echo "✗ FreeSWITCH installation may be incomplete"
    echo ""
    echo "If PAT was not provided during build, FreeSWITCH was not installed."
    echo "Rebuild the container with PAT set in .devcontainer/.env"
fi





