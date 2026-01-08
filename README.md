# Dialer
- Rust dialer domain controller
- FreeSWITCH telephony engine
- Next JS UI
   - Supabase auth flows already in place
- Supabase back end
   - Setup with personal and team account support
   - Beginning of support for Stripe integration
   - Supabase auth flows already in place

## Coffee
Before you continue, consider 

[!["Buy Me A Coffee"](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://buymeacoffee.com/jeremygarr5)

## Development Setup

### Prerequisites
- [VS Code](https://code.visualstudio.com/) with the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
- [Docker](https://www.docker.com/get-started) installed and running

### Installation

1. **Open the dev container**
   - Open this project in VS Code
   - When prompted, click "Reopen in Container" or use Command Palette (ctrl + shift + p) → "Dev Containers: Reopen in Container"

2. **Install up FreeSWITCH**
   - Copy the example environment file:
     ```bash
     cp dialer/tools/.env.example dialer/tools/.env
     ```
   
   - Get your FreeSWITCH Personal Access Token (PAT):
     - Visit https://freeswitch.org/fsget
     - Follow the instructions to obtain your PAT
   
   - Edit `dialer/tools/.env` and add your PAT:
     ```bash
     PAT=your_actual_token_here
     ```
   
   - Install FreeSWITCH:
     ```bash
     ./dialer/tools/fs-up
     ```
   
   - Verify the installation:
     ```bash
     ./dialer/tools/check-freeswitch.sh
     ```
     This script will check if FreeSWITCH is properly installed and show you the installed packages, modules, and configuration.


That's it! Your development environment is ready.


## Running the dialer

1. **Run FreeSWITCH**
   - Start FreeSWITCH in development mode:
     ```bash
     ./dialer/tools/fs-run
     ```
   
   - The script will:
     - Run FreeSWITCH as the `freeswitch` user (created in the devcontainer)
     - Use the vanilla configuration from `/usr/share/freeswitch/conf/vanilla/`
     - Run in the foreground so you can see logs directly in your terminal
     - Use development-friendly flags (`-nonat`, `-nosql`)
   
   - To stop FreeSWITCH, press `Ctrl+C` in the terminal where it's running


2. **Run dialer app**
   - First, copy the example environment file:
     ```bash
     cp dialer/.env.example dialer/.env
     ```
   
   - Edit `dialer/.env` and update the FreeSWITCH connection settings if needed (default values should work for local development):
     - `FREESWITCH_HOST`: FreeSWITCH hostname (default: `localhost`)
     - `FREESWITCH_PORT`: FreeSWITCH ESL port (default: `8021`)
     - `FREESWITCH_PASSWORD`: FreeSWITCH ESL password (default: `ClueCon`)
   
   - Run the dialer worker:
     ```bash
     cargo run --bin dialer_worker
     ```
   
   - The dialer worker will:
     - Connect to FreeSWITCH via Event Socket Library (ESL)
     - Subscribe to telephony events (call originate, answer, hangup, etc.)
     - Process call events and manage call state
   
   - To stop the dialer worker, press `Ctrl+C` in the terminal where it's running
   
   - Note: Make sure FreeSWITCH is running (step 1) before starting the dialer worker, as it needs to connect to the FreeSWITCH ESL socket
