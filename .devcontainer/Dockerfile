FROM mcr.microsoft.com/devcontainers/base:jammy

# ---------- System packages (psql + basics) ----------
USER root

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        curl \
        ca-certificates \
        postgresql-client && \
    rm -rf /var/lib/apt/lists/*

# ---------- Switch to non-root dev user ----------
USER vscode
SHELL ["/bin/bash", "-lc"]

# ---------- Install nvm + Node LTS (+ npm) ----------
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash && \
    source "$HOME/.nvm/nvm.sh" && \
    nvm install --lts && \
    nvm alias default "lts/*"

# Make nvm/Node available in all interactive shells
RUN echo 'export NVM_DIR="$HOME/.nvm"' >> ~/.bashrc && \
    echo '[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"' >> ~/.bashrc

# ---------- Install bun ----------
RUN curl -fsSL https://bun.sh/install | bash && \
    echo 'export BUN_INSTALL="$HOME/.bun"' >> ~/.bashrc && \
    echo 'export PATH="$BUN_INSTALL/bin:$PATH"' >> ~/.bashrc
