# Slice 2: Inbound Call + Softphone

## Purpose

Test scenario for inbound calls with softphone registration.

## Test Flow

1. **Softphone Registration**: A softphone (e.g., Linphone) registers as user `1001` with password `1234`
2. **Inbound Call**: An inbound call arrives at DID `18005550100`
3. **Call Routing**: FreeSWITCH bridges the call to the registered softphone
4. **Event Monitoring**: ESL monitors call lifecycle events (CHANNEL_CREATE, CHANNEL_ANSWER, CHANNEL_HANGUP)

## Configuration Changes from Vanilla

### `autoload_configs/event_socket.conf.xml`

ESL configuration for external application control:

```xml
<configuration name="event_socket.conf" description="Socket Client">
  <settings>
    <param name="nat-map" value="false"/>
    <param name="listen-ip" value="0.0.0.0"/>
    <param name="listen-port" value="8021"/>
    <param name="password" value="ClueCon"/>
  </settings>
</configuration>
```

**Changes:**
- Listens on all interfaces (`0.0.0.0`) for ESL connections
- Port 8021 (standard ESL port)
- Password: `ClueCon` (default, change in production)

### `sip_profiles/internal.xml`

SIP profile configuration for softphone registration.

**Key settings:**
- Allows SIP registration from softphones
- Configures RTP/SIP parameters for local network testing
- Uses vanilla directory configuration for user authentication

## Usage

```bash
# In your .env file
PAT=your-token-here
OVERLAY=slice-2-inbound-softphone

# Build and run
docker compose build
docker compose up -d

# Verify overlay was applied
docker compose logs | grep "Applying configuration overlay"
```

## Testing

### 1. Register Softphone

Configure Linphone (or similar):
- Username: `1001`
- Password: `1234`
- Domain: `127.0.0.1`
- Transport: UDP

### 2. Verify Registration

```bash
docker exec -it fs-freeswitch-1 fs_cli -x "sofia status profile internal reg"
```

### 3. Simulate Inbound Call

```bash
docker exec -it fs-freeswitch-1 fs_cli -x \
  "originate {origination_caller_id_number=15551234567}loopback/18005550100/public &park"
```

### 4. Monitor Events

```bash
cd tools/
python3 esl_logger.py
```

## Expected Behavior

1. Softphone successfully registers
2. Inbound call rings the softphone
3. Answer establishes 2-way audio
4. ESL events captured throughout lifecycle

## Next Steps

- Add dialplan configuration for the DID routing
- Configure user directory entries
- Set up proper authentication
