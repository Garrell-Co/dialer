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
- VS Code with the Dev Containers extension
- Docker

### Installation

1. **Open the dev container**
   - Open this project in VS Code
   - When prompted, click "Reopen in Container" or use Command Palette → "Dev Containers: Reopen in Container"

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

That's it! Your development environment is ready.
