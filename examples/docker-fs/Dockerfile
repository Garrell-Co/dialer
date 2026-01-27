FROM debian:12-slim

ARG PAT

ENV DEBIAN_FRONTEND=noninteractive

# Install basic dependencies (matching devcontainer pattern)
RUN apt-get update && apt-get install -y \
    curl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install FreeSWITCH if PAT is provided (following fs-up script method)
RUN if [ -n "$PAT" ]; then \
      echo "Installing FreeSWITCH with PAT (length: ${#PAT} chars)..." && \
      curl -sSL https://freeswitch.org/fsget | bash -s "$PAT" release install && \
      apt-get clean && \
      rm -rf /var/lib/apt/lists/* && \
      echo "Verifying FreeSWITCH installation..." && \
      (which freeswitch && echo "FreeSWITCH binary found: $(which freeswitch)" || (echo "ERROR: FreeSWITCH binary not found!" && exit 1)) && \
      echo "Checking FreeSWITCH packages..." && \
      dpkg -l | grep "^ii.*freeswitch" | head -3; \
    else \
      echo "WARNING: PAT not provided. FreeSWITCH installation skipped."; \
    fi

# Ensure directories exist with proper ownership (fsget script already creates freeswitch user/group)
# Also create database directory that FreeSWITCH needs
RUN mkdir -p /var/run/freeswitch /var/log/freeswitch /etc/freeswitch /var/lib/freeswitch/db && \
    (id freeswitch >/dev/null 2>&1 && chown -R freeswitch:freeswitch /var/run/freeswitch /var/log/freeswitch /etc/freeswitch /var/lib/freeswitch || true)

# Copy entrypoint script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Use vanilla configuration from /usr/share/freeswitch/conf/vanilla/ (matching fs-run behavior)
# -nc: no console (run in background)
# -nonat: don't require NAT traversal
# -conf, -log, -db: must all be specified together
ENV FS_CONF_DIR=/usr/share/freeswitch/conf/vanilla
ENV FS_LOG_DIR=/var/log/freeswitch
ENV FS_DB_DIR=/var/lib/freeswitch/db

ENTRYPOINT ["/entrypoint.sh"]
