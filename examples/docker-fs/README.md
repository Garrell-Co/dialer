# FreeSWITCH Vertical Slice 2

This directory contains the FreeSWITCH stack for Vertical Slice 2 (Inbound Call + Softphone).

## Prerequisites

- Docker
- SignalWire Personal Access Token (PAT) for installing FreeSWITCH (or use a public image if available).

## Setup

1. **Build the Image**

   ```bash
   cd services/fs
   # Replace YOUR_TOKEN with your SignalWire PAT
   docker build --build-arg SIGNALWIRE_TOKEN=YOUR_TOKEN -t my-freeswitch .
   ```

   *Note: If you do not have a token, you may need to adjust `docker-compose.yml` to use a public image like `signalwire/freeswitch` instead of `build: .`*

2. **Run the Stack**

   ```bash
   docker compose up -d
   ```

   This runs FreeSWITCH with `network_mode: host`.
   - SIP: 5060/5080 (UDP/TCP)
   - ESL: 8021 (TCP)
   - RTP: Range (usually 16384-32768)

3. **Verify**

   Check logs:
   ```bash
   docker compose logs -f freeswitch
   ```

   Check status:
   ```bash
   docker exec -it fs-freeswitch-1 fs_cli -x "status"
   ```

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
cd services/fs/tools
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
cd services/fs/tools
./test_sip_registration.sh 1001 1234 127.0.0.1 5060
```

### Verify Registration

After attempting registration, verify it worked:

```bash
# Check registrations
docker exec -it fs-freeswitch-1 fs_cli -x "sofia status profile internal reg"

# Check logs for registration events
docker compose logs freeswitch | grep -i register
```

## Simulate Inbound Call

To simulate an inbound call to DID `18005550100`:

```bash
docker exec -it fs-freeswitch-1 fs_cli -x "originate {origination_caller_id_number=15551234567}loopback/18005550100/public &park"
```

- If Agent 1001 is **registered**: The softphone should ring. Answer it to establish 2-way audio (echo test or silence depending on bridge).
- If Agent 1001 is **unregistered**: You should hear a fallback tone/message in the `fs_cli` output or logs (and the call will hangup).

## Lifecycle Logging

A Python script is provided to log call lifecycle events to `services/fs/data/call_lifecycle.jsonl`.

1. **Run the Logger** (on host):

   ```bash
   cd services/fs/tools
   python3 esl_logger.py
   ```

2. **Inspect Logs**:

   ```bash
   tail -f services/fs/data/call_lifecycle.jsonl
   ```

   You should see JSON events for `CHANNEL_CREATE`, `CHANNEL_ANSWER`, `CHANNEL_HANGUP_COMPLETE` with UUID, DID, and Agent fields.
