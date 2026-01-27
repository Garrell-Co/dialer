#!/bin/bash
# Setup development environment

echo "Setting up FreeSWITCH tools in PATH..."
mkdir -p ~/bin

# Create symlinks for FreeSWITCH tools
ln -sf /workspaces/dialer/.devcontainer/tools/fs-run ~/bin/fs-run
ln -sf /workspaces/dialer/.devcontainer/tools/fs-up ~/bin/fs-up
ln -sf /workspaces/dialer/.devcontainer/tools/check-freeswitch.sh ~/bin/check-freeswitch
ln -sf /workspaces/dialer/.devcontainer/tools/check-fs-installation.sh ~/bin/check-fs-installation

echo "✓ FreeSWITCH tools available: fs-run, fs-up, check-freeswitch"