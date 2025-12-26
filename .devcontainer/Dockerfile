FROM debian:12-slim

ENV DEBIAN_FRONTEND=noninteractive

# Install basic tools and pyenv dependencies
RUN apt-get update && apt-get install -y \
    git \
        curl \
    procps \
    vim \
    make \
    build-essential \
    libssl-dev \
    zlib1g-dev \
    libbz2-dev \
    libreadline-dev \
    libsqlite3-dev \
    libncurses5-dev \
    libncursesw5-dev \
    xz-utils \
    tk-dev \
    libffi-dev \
    liblzma-dev \
    locales \
    zsh \
    fuse \
    libgl1 \
    libgl1-mesa-glx \
    libglu1-mesa \
    libx11-6 \
    libx11-xcb1 \
    libxcb1 \
    libxcb-xinerama0 \
    libxcb-xkb1 \
    libxkbcommon0 \
    libxkbcommon-x11-0 \
    libasound2 \
    && rm -rf /var/lib/apt/lists/*

# Set up locale
RUN sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && \
    locale-gen && \
    update-locale LANG=en_US.UTF-8

ENV LANG=en_US.UTF-8 \
    LC_ALL=en_US.UTF-8

# Install sudo and create non-root user
RUN apt-get update && apt-get install -y sudo && rm -rf /var/lib/apt/lists/* && \
    groupadd --gid 1000 vscode && \
    useradd --uid 1000 --gid vscode --shell /bin/zsh --create-home vscode && \
    echo "vscode ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Switch to vscode user and install pyenv
USER vscode
WORKDIR /home/vscode

# Install pyenv (before oh-my-zsh to avoid .zshrc overwrite)
RUN curl https://pyenv.run | bash

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Install oh-my-zsh with candy theme
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" "" --unattended && \
    sed -i 's/ZSH_THEME="robbyrussell"/ZSH_THEME="candy"/' ~/.zshrc

# Configure pyenv, Rust, and PATH in .zshrc (after oh-my-zsh installation)
RUN echo '' >> ~/.zshrc && \
    echo '# Pyenv configuration' >> ~/.zshrc && \
    echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/.zshrc && \
    echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/.zshrc && \
    echo 'eval "$(pyenv init -)"' >> ~/.zshrc && \
    echo '' >> ~/.zshrc && \
    echo '# Rust/Cargo configuration' >> ~/.zshrc && \
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc && \
    echo '' >> ~/.zshrc && \
    echo '# Add ~/bin to PATH' >> ~/.zshrc && \
    echo 'export PATH="$HOME/bin:$PATH"' >> ~/.zshrc