#!/bin/sh
set -e

FS_CONF_DIR="${FS_CONF_DIR:-/usr/share/freeswitch/conf/vanilla}"
FS_LOG_DIR="${FS_LOG_DIR:-/var/log/freeswitch}"
FS_DB_DIR="${FS_DB_DIR:-/var/lib/freeswitch/db}"

# Ensure directories exist
mkdir -p "$FS_LOG_DIR" "$FS_DB_DIR"

# Ensure TLS certificate directory exists for auto-generated certificates
# This is needed when /etc/freeswitch is mounted from a volume
if [ -d "/etc/freeswitch" ]; then
  mkdir -p /etc/freeswitch/tls
fi

# Run as freeswitch user if it exists, otherwise as root
# Remove -nc flag so FreeSWITCH runs in foreground (required for Docker)
if id freeswitch >/dev/null 2>&1; then
  chown -R freeswitch:freeswitch "$FS_LOG_DIR" "$FS_DB_DIR" 2>/dev/null || true
  # Ensure TLS directory has correct ownership for certificate auto-generation
  [ -d "/etc/freeswitch/tls" ] && chown -R freeswitch:freeswitch /etc/freeswitch/tls 2>/dev/null || true
  exec su -s /bin/sh freeswitch -c "exec freeswitch -nonat -conf \"$FS_CONF_DIR\" -log \"$FS_LOG_DIR\" -db \"$FS_DB_DIR\""
else
  exec freeswitch -nonat -conf "$FS_CONF_DIR" -log "$FS_LOG_DIR" -db "$FS_DB_DIR"
fi

