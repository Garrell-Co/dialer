# Configuration Overlay System - Quick Reference

## Overview

The overlay system allows you to maintain **only the configuration files that differ** from FreeSWITCH vanilla, making it easy to:
- See exactly what each test scenario requires
- Compare configurations between slices
- Track minimal changes in git

## Quick Start

### 1. Set Up Environment

```bash
cd examples/docker-fs
cp .env.example .env
# Edit .env and add your PAT and desired OVERLAY
```

Example `.env`:
```bash
PAT=your-freeswitch-pat-here
OVERLAY=slice-2-inbound-softphone
```

### 2. Build and Run

```bash
docker compose build --no-cache
docker compose up -d
```

### 3. Verify

```bash
# Check that overlay was applied
docker compose logs | grep "overlay"

# Should see:
# ==> Applying configuration overlay: slice-2-inbound-softphone
# ==> Copying overlay files from /overlays/slice-2-inbound-softphone
# ==> Using merged configuration at /tmp/freeswitch-conf
```

## Directory Structure

```
examples/docker-fs/
├── conf-overlays/              # All configuration overlays
│   ├── README.md              # Overview of all slices
│   └── slice-2-inbound-softphone/
│       ├── README.md          # Slice-specific docs
│       ├── autoload_configs/  # Only changed configs
│       │   └── event_socket.conf.xml
│       └── sip_profiles/      # Only changed profiles
│           └── internal.xml
├── docker-compose.yml         # References OVERLAY env var
├── entrypoint.sh              # Merges overlay at runtime
├── Dockerfile                 # Builds FreeSWITCH image
└── .env                       # Your config (gitignored)
```

## Working with Overlays

### Use a Different Slice

```bash
# Option 1: Update .env file
echo "OVERLAY=slice-3-outbound-bridge" >> .env
docker compose restart

# Option 2: Inline environment variable
OVERLAY=slice-3-outbound-bridge docker compose up -d
```

### Use Vanilla (No Overlay)

```bash
# Option 1: Clear OVERLAY in .env
echo "OVERLAY=" > .env
docker compose restart

# Option 2: Unset the variable
unset OVERLAY
docker compose up -d
```

### Create a New Slice

```bash
# 1. Create directory structure
mkdir -p conf-overlays/slice-3-my-test/{autoload_configs,dialplan}

# 2. Add only the files that differ from vanilla
cp /usr/share/freeswitch/conf/vanilla/dialplan/default.xml \
   conf-overlays/slice-3-my-test/dialplan/default.xml

# 3. Edit the file with your changes
vim conf-overlays/slice-3-my-test/dialplan/default.xml

# 4. Document what you changed
echo "# Slice 3: My Test" > conf-overlays/slice-3-my-test/README.md

# 5. Test it
OVERLAY=slice-3-my-test docker compose up -d
```

## Comparing Slices

### See What's Different Between Slices

```bash
# Compare two slices
diff -r conf-overlays/slice-2-inbound-softphone/ \
        conf-overlays/slice-3-outbound-bridge/

# Compare overlay to vanilla (inside container)
docker exec -it fs-freeswitch-1 bash
diff -r /usr/share/freeswitch/conf/vanilla/ \
        /tmp/freeswitch-conf/
```

### View Overlay Files

```bash
# List all files in an overlay
find conf-overlays/slice-2-inbound-softphone/ -type f

# See what an overlay changes
tree conf-overlays/slice-2-inbound-softphone/
```

## How It Works

1. **Base**: Vanilla FreeSWITCH config at `/usr/share/freeswitch/conf/vanilla/`
2. **Overlay**: Your slice-specific files in `conf-overlays/<slice-name>/`
3. **Merge**: `entrypoint.sh` copies vanilla → temp dir → overlays your files
4. **Runtime**: FreeSWITCH uses the merged config

## Benefits

✅ **Minimal Storage** - Only track changed files, not 100+ vanilla files  
✅ **Clear Diffs** - Easy to see what each test needs  
✅ **Git Friendly** - Small commits showing actual changes  
✅ **Composable** - Could layer multiple overlays if needed  
✅ **Documented** - Directory structure shows purpose  

## Troubleshooting

### Overlay Not Applied

```bash
# Check if overlay directory exists
ls -la conf-overlays/

# Check if OVERLAY env var is set
docker compose config | grep OVERLAY

# Check container logs
docker compose logs | grep -i overlay
```

### Wrong Config Being Used

```bash
# Verify the merged config inside container
docker exec -it fs-freeswitch-1 ls -la /tmp/freeswitch-conf

# Check a specific file
docker exec -it fs-freeswitch-1 cat /tmp/freeswitch-conf/autoload_configs/event_socket.conf.xml
```

### Want to Use Full Custom Config

If you need a complete custom config (not an overlay):

```bash
# 1. Remove the OVERLAY env var
OVERLAY= docker compose up -d

# 2. Mount your config directly (edit docker-compose.yml)
volumes:
  - ./my-custom-conf:/etc/freeswitch

# 3. Update entrypoint to use it
environment:
  - FS_CONF_DIR=/etc/freeswitch
```

## Related Documentation

- `conf-overlays/README.md` - Overview of all slices
- `conf-overlays/slice-*/README.md` - Slice-specific documentation
- `README.md` - Main FreeSWITCH Docker setup guide
