# FreeSWITCH Configuration Overlays

This directory contains configuration overlays for different vertical slices. Each overlay contains **only the files that differ** from the vanilla FreeSWITCH configuration at `/usr/share/freeswitch/conf/vanilla/`.

## How It Works

1. **Base**: FreeSWITCH vanilla configuration (installed by the package)
2. **Overlay**: Files in a slice directory override corresponding vanilla files
3. **Merge**: The entrypoint script merges base + overlay at runtime

This approach means we only track the **delta** from vanilla, making it clear what each test scenario requires.

## Available Slices

### `slice-2-inbound-softphone/`

**Purpose**: Test inbound calls with softphone registration

**Changes from vanilla:**
- `autoload_configs/event_socket.conf.xml` - ESL configuration for external control
- `sip_profiles/internal.xml` - SIP profile for softphone registration

**Test scenario:**
1. Softphone (e.g., Linphone) registers as user 1001
2. Inbound call arrives at DID 18005550100
3. Call bridges to registered softphone
4. ESL monitors call lifecycle events

**Usage:**
```bash
OVERLAY=slice-2-inbound-softphone docker compose up -d
```

## Adding New Slices

To create a new slice overlay:

1. **Create directory**: `mkdir -p conf-overlays/slice-N-description/`
2. **Add only changed files**: Copy the file structure from vanilla, include only modified files
3. **Document**: Update this README with the slice purpose and changes
4. **Test**: Run with `OVERLAY=slice-N-description docker compose up`

Example structure:
```
conf-overlays/
└── slice-3-outbound-bridge/
    ├── dialplan/
    │   └── default.xml          # Only this file differs from vanilla
    └── autoload_configs/
        └── sofia.conf.xml       # Only this file differs from vanilla
```

## Comparing Slices

To see what's different between slices:

```bash
# Compare two slices
diff -r slice-2-inbound-softphone/ slice-3-outbound-bridge/

# See what a slice changes from vanilla (requires vanilla config available)
diff -r /usr/share/freeswitch/conf/vanilla/ slice-2-inbound-softphone/
```

## Default Behavior

If no `OVERLAY` environment variable is set, the container uses the vanilla configuration directly without any modifications.
