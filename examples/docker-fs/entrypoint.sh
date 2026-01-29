#!/bin/sh
set -e

# Configuration overlay support
OVERLAY="${OVERLAY:-}"
BASE_CONF="/usr/share/freeswitch/conf/vanilla"
OVERLAY_BASE="/overlays"
MERGED_CONF="/tmp/freeswitch-conf"

# Standard directories
FS_LOG_DIR="${FS_LOG_DIR:-/var/log/freeswitch}"
FS_DB_DIR="${FS_DB_DIR:-/var/lib/freeswitch/db}"

# Ensure directories exist
mkdir -p "$FS_LOG_DIR" "$FS_DB_DIR"

# Determine configuration directory
if [ -n "$OVERLAY" ] && [ -d "$OVERLAY_BASE/$OVERLAY" ]; then
  echo "==> Applying configuration overlay: $OVERLAY"
  
  # Create merged configuration
  rm -rf "$MERGED_CONF"
  cp -r "$BASE_CONF" "$MERGED_CONF"
  
  # Overlay the slice-specific files
  echo "==> Copying overlay files from $OVERLAY_BASE/$OVERLAY"
  cp -r "$OVERLAY_BASE/$OVERLAY"/* "$MERGED_CONF"/
  
  # Ensure TLS directory exists in merged config
  mkdir -p "$MERGED_CONF/tls"
  
  # Set permissions if freeswitch user exists
  if id freeswitch >/dev/null 2>&1; then
    chown -R freeswitch:freeswitch "$MERGED_CONF" 2>/dev/null || true
  fi
  
  FS_CONF_DIR="$MERGED_CONF"
  echo "==> Using merged configuration at $FS_CONF_DIR"
else
  if [ -n "$OVERLAY" ]; then
    echo "==> WARNING: Overlay '$OVERLAY' not found at $OVERLAY_BASE/$OVERLAY"
    echo "==> Falling back to vanilla configuration"
  else
    echo "==> Using vanilla configuration (no overlay specified)"
  fi
  FS_CONF_DIR="$BASE_CONF"
fi

# Run as freeswitch user if it exists, otherwise as root
# Remove -nc flag so FreeSWITCH runs in foreground (required for Docker)
if id freeswitch >/dev/null 2>&1; then
  chown -R freeswitch:freeswitch "$FS_LOG_DIR" "$FS_DB_DIR" 2>/dev/null || true
  exec su -s /bin/sh freeswitch -c "exec freeswitch -nonat -conf \"$FS_CONF_DIR\" -log \"$FS_LOG_DIR\" -db \"$FS_DB_DIR\""
else
  exec freeswitch -nonat -conf "$FS_CONF_DIR" -log "$FS_LOG_DIR" -db "$FS_DB_DIR"
fi

