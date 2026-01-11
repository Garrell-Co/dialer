FROM debian:bookworm

ARG SIGNALWIRE_TOKEN

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl \
    gnupg2 \
    lsb-release \
    wget \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install FreeSWITCH if token is provided
RUN if [ -n "$SIGNALWIRE_TOKEN" ]; then \
      curl -u signalwire:${SIGNALWIRE_TOKEN} -sSL https://freeswitch.signalwire.com/repo/deb/debian-release/signalwire-freeswitch-repo.gpg | gpg --dearmor -o /usr/share/keyrings/signalwire-freeswitch-repo.gpg && \
      echo "deb [signed-by=/usr/share/keyrings/signalwire-freeswitch-repo.gpg] https://freeswitch.signalwire.com/repo/deb/debian-release/ `lsb_release -sc` main" > /etc/apt/sources.list.d/freeswitch.list && \
      echo "machine freeswitch.signalwire.com login signalwire password ${SIGNALWIRE_TOKEN}" > /etc/apt/auth.conf.d/freeswitch.conf && \
      apt-get update && \
      apt-get install -y freeswitch-meta-all && \
      apt-get clean && \
      rm -rf /var/lib/apt/lists/*; \
    else \
      echo "WARNING: SIGNALWIRE_TOKEN not provided. FreeSWITCH installation skipped."; \
    fi

# Ensure config dir exists
RUN mkdir -p /etc/freeswitch

CMD ["/usr/bin/freeswitch", "-nc", "-nf", "-nonat"]
