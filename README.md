# Dialer
Learning and exploring the world of telephony development

## Development Setup

### Prerequisites
- [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
- [Docker](https://www.docker.com/get-started) installed and running

### Installation

1. **Open the dev container**
   - Open this project in VS Code
   - When prompted, click "Reopen in Container" or use Command Palette (ctrl + shift + p) → "Dev Containers: Reopen in Container"

2. **Set up FreeSWITCH**
   - Copy the example environment file:
     ```bash
     cp tools/.env.example tools/.env
     ```
   
   - Get your FreeSWITCH Personal Access Token (PAT):
     - Visit https://freeswitch.org/fsget
     - Follow the instructions to obtain your PAT
   
   - Edit `tools/.env` and add your PAT:
     ```bash
     PAT=your_actual_token_here
     ```
   
   - Install FreeSWITCH:
     ```bash
     ./tools/fs-up
     ```
   
   - Verify the installation:
     ```bash
     ./tools/check-freeswitch.sh
     ```
     This script will check if FreeSWITCH is properly installed and show you the installed packages, modules, and configuration.

3. **Run FreeSWITCH**
   - Start FreeSWITCH in development mode:
     ```bash
     ./tools/fs-run
     ```
   
   - The script will:
     - Run FreeSWITCH as the `freeswitch` user (created in the devcontainer)
     - Use the vanilla configuration from `/usr/share/freeswitch/conf/vanilla/`
     - Run in the foreground so you can see logs directly in your terminal
     - Use development-friendly flags (`-nonat`, `-nosql`)
   
   - To stop FreeSWITCH, press `Ctrl+C` in the terminal where it's running

That's it! Your development environment is ready.
