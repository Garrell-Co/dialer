# FreeSWITCH Docker Example

This directory contains a Docker setup for FreeSWITCH for testing and development.

## Prerequisites

- Docker and Docker Compose
- FreeSWITCH Personal Access Token (PAT) from https://freeswitch.org/

## Setup

1. **Configure Environment**

   Copy the example environment file and add your FreeSWITCH PAT:

   ```bash
   cd examples/docker-fs
   cp .env.example .env
   # Edit .env and add your PAT
   ```

   Your `.env` file should contain:
   ```bash
   PAT=your-freeswitch-personal-access-token-here
   OVERLAY=slice-2-inbound-softphone
   ```

   **Note**: The `.env` file is gitignored to protect your credentials.
   
   Set `OVERLAY` to the vertical slice you want to test (leave empty for vanilla config).

2. **Build the Image**

   Docker Compose will automatically read the `PAT` from your `.env` file:

   ```bash
   docker compose build --no-cache
   ```

   During the build, you should see output confirming FreeSWITCH installation:
   ```
   Installing FreeSWITCH with PAT (length: XX chars)...
   FreeSWITCH binary found: /usr/bin/freeswitch
   ```

3. **Run the Stack**

   ```bash
   docker compose up -d
   ```

   This runs FreeSWITCH with `network_mode: host` for direct network access:
   - SIP: 5060/5080 (UDP/TCP)
   - ESL: 8021 (TCP)
   - RTP: Range (usually 16384-32768)

4. **Verify**

   Check logs:
   ```bash
   docker compose logs -f
   ```

   Check status:
   ```bash
   docker exec -it fs-freeswitch-1 fs_cli -x "status"
   ```

   If you see "freeswitch: not found" errors, verify that:
   - Your PAT is correct in the `.env` file
   - You ran `docker compose build --no-cache` after setting the PAT

## Configuration

The FreeSWITCH container uses a **configuration overlay system** that allows you to specify only the files that differ from vanilla configuration.

📖 **See [OVERLAY-USAGE.md](./OVERLAY-USAGE.md) for a complete guide** on working with overlays.

**Directory Structure:**
- `./conf-overlays/` - Configuration overlays for different vertical slices
  - Each subdirectory contains only the files that differ from vanilla
  - See `conf-overlays/README.md` for details on each slice
- `./data/` - FreeSWITCH logs (mounted to `/var/log/freeswitch`)
- `./tools/` - Helper scripts for testing

**Using Configuration Overlays:**

1. **List available overlays:**
   ```bash
   ls conf-overlays/
   ```

2. **Use a specific overlay:**
   
   Set the `OVERLAY` environment variable in your `.env` file:
   ```bash
   OVERLAY=slice-2-inbound-softphone
   ```
   
   Or specify it when running:
   ```bash
   OVERLAY=slice-2-inbound-softphone docker compose up -d
   ```

3. **Use vanilla configuration (no overlay):**
   
   Leave `OVERLAY` empty or unset in your `.env` file.

**How It Works:**

1. The entrypoint script starts with vanilla FreeSWITCH config from `/usr/share/freeswitch/conf/vanilla/`
2. If `OVERLAY` is set, it copies vanilla to a temp directory and overlays your slice-specific files
3. FreeSWITCH runs with the merged configuration

This approach means you only maintain the **delta** from vanilla, making it clear what each test scenario requires.

**Creating New Overlays:**

See `conf-overlays/README.md` for instructions on creating new vertical slice configurations.

## Softphone Setup (Linphone)

1. **Install Linphone** on your host (Ubuntu).
2. **Configure Account**:
   - **Username**: 1001
   - **SIP Domain**: 127.0.0.1 (or your host LAN IP)
   - **Password**: 1234
   - **Transport**: UDP or TCP
3. **Register**: You should see "Registered" in Linphone and in FreeSWITCH logs.

## Testing SIP Registration (Without Linphone)

### Method 1: Check Registration Status via fs_cli

Check if a user is currently registered:

```bash
docker exec -it fs-freeswitch-1 fs_cli -x "sofia status profile internal reg"
```

This shows all active registrations. Look for your username (e.g., `1001`).

### Method 2: Python SIP Registration Script

Use the provided Python script to test registration:

```bash
cd examples/docker-fs/tools
python3 test_sip_register.py 1001 1234 127.0.0.1 5060
```

This will:
- Send a REGISTER request
- Handle authentication (401 challenge)
- Confirm successful registration (200 OK)

### Method 3: Command-Line SIP Tools

**Using sipsak** (if installed):
```bash
sudo apt-get install sipsak
sipsak -U -s sip:1001@127.0.0.1:5060 -u 1001 -a 1234 -H 127.0.0.1 -p 5060 -v
```

**Using sipcmd** (if installed):
```bash
sudo apt-get install sipcmd
sipcmd -u 1001 -c 1234 -P sip -w 127.0.0.1 -x "c" -x "h"
```

### Method 4: All-in-One Test Script

Run the comprehensive test script that tries multiple methods:

```bash
cd examples/docker-fs/tools
./test_sip_registration.sh 1001 1234 127.0.0.1 5060
```

### Verify Registration

After attempting registration, verify it worked:

```bash
# Check registrations
docker exec -it fs-freeswitch-1 fs_cli -x "sofia status profile internal reg"

# Check logs for registration events
docker compose logs | grep -i register
```

## Simulate Inbound Call

To simulate an inbound call to DID `18005550100`:

```bash
docker exec -it fs-freeswitch-1 fs_cli -x "originate {origination_caller_id_number=15551234567}loopback/18005550100/public &park"
```

- If Agent 1001 is **registered**: The softphone should ring. Answer it to establish 2-way audio (echo test or silence depending on bridge).
- If Agent 1001 is **unregistered**: You should hear a fallback tone/message in the `fs_cli` output or logs (and the call will hangup).

## Lifecycle Logging

A Python script is provided to log call lifecycle events to `examples/docker-fs/data/call_lifecycle.jsonl`.

1. **Run the Logger** (on host):

   ```bash
   cd examples/docker-fs/tools
   python3 esl_logger.py
   ```

2. **Inspect Logs**:

   ```bash
   tail -f examples/docker-fs/data/call_lifecycle.jsonl
   ```

   You should see JSON events for `CHANNEL_CREATE`, `CHANNEL_ANSWER`, `CHANNEL_HANGUP_COMPLETE` with UUID, DID, and Agent fields.

## Troubleshooting

### Container Fails to Start: "freeswitch: not found"

This means FreeSWITCH wasn't installed during the build:

1. Verify your `.env` file exists and contains a valid PAT:
   ```bash
   cat .env
   ```

2. Rebuild with no cache to ensure the PAT is used:
   ```bash
   docker compose down
   docker compose build --no-cache
   docker compose up -d
   ```

3. Check build logs for FreeSWITCH installation confirmation:
   ```bash
   docker compose build --no-cache 2>&1 | grep -i freeswitch
   ```

### Container Starts but FreeSWITCH Won't Load: "Cannot Open log directory or XML Root!"

If you manually run `freeswitch` inside the container without arguments, it will fail. Use one of these approaches:

1. **Let the entrypoint handle it** (recommended):
   ```bash
   docker compose up -d
   ```

2. **Or provide the required arguments manually**:
   ```bash
   docker exec -it fs-freeswitch-1 \
     freeswitch -nonat \
     -conf /usr/share/freeswitch/conf/vanilla \
     -log /var/log/freeswitch \
     -db /var/lib/freeswitch/db
   ```

### Can't Connect to FreeSWITCH CLI

Verify the container is running and FreeSWITCH is started:

```bash
docker compose ps
docker compose logs | tail -20
```

Try connecting with verbose output:
```bash
docker exec -it fs-freeswitch-1 fs_cli -H 127.0.0.1 -P 8021 -p ClueCon
```

### SIP Registration Fails

1. Check if FreeSWITCH is listening on port 5060:
   ```bash
   docker exec -it fs-freeswitch-1 fs_cli -x "sofia status"
   ```

2. Verify network_mode is set to "host" in docker-compose.yml

3. Check FreeSWITCH logs for authentication errors:
   ```bash
   docker compose logs | grep -i "auth\|register"
   ```
