#!/bin/bash
# Test SIP registration without a softphone
# Usage: ./test_sip_registration.sh [username] [password] [server] [port]

USERNAME="${1:-1001}"
PASSWORD="${2:-1234}"
SERVER="${3:-127.0.0.1}"
PORT="${4:-5060}"

echo "Testing SIP registration for user: $USERNAME@$SERVER:$PORT"
echo ""

# Method 1: Check registration status via fs_cli (if already registered)
echo "=== Method 1: Check existing registrations ==="
docker exec -it fs-freeswitch-1 fs_cli -x "sofia status profile internal reg" 2>/dev/null | grep -i "$USERNAME" || echo "No active registration found for $USERNAME"
echo ""

# Method 2: Use sipsak if available
if command -v sipsak >/dev/null 2>&1; then
    echo "=== Method 2: Using sipsak ==="
    echo "Registering $USERNAME..."
    sipsak -U -s sip:$USERNAME@$SERVER:$PORT -u $USERNAME -a $PASSWORD -H $SERVER -p $PORT -v
    echo ""
    echo "Checking registration..."
    sipsak -U -s sip:$USERNAME@$SERVER:$PORT -u $USERNAME -a $PASSWORD -H $SERVER -p $PORT -I
else
    echo "=== Method 2: sipsak not installed ==="
    echo "Install with: sudo apt-get install sipsak"
    echo ""
fi

# Method 3: Use sipcmd if available
if command -v sipcmd >/dev/null 2>&1; then
    echo "=== Method 3: Using sipcmd ==="
    sipcmd -u $USERNAME -c $PASSWORD -P sip -w $SERVER -x "c" -x "h"
else
    echo "=== Method 3: sipcmd not installed ==="
    echo "Install with: sudo apt-get install sipcmd"
    echo ""
fi

# Method 4: Python pjsua (if available)
if python3 -c "import pjsua" 2>/dev/null; then
    echo "=== Method 4: Using Python pjsua ==="
    python3 <<EOF
import pjsua as pj
import sys

def log_cb(level, str, len):
    print(str, end='')

# Create library instance
lib = pj.Lib()

try:
    # Initialize library
    lib.init(log_cfg = pj.LogConfig(level=3, callback=log_cb))
    
    # Create UDP transport
    transport = lib.create_transport(pj.TransportType.UDP, pj.TransportConfig(5080))
    
    # Start library
    lib.start()
    
    # Create account
    acc_cfg = pj.AccountConfig("$SERVER", "$USERNAME", "$PASSWORD")
    acc_cfg.id = "sip:$USERNAME@$SERVER"
    acc_cfg.reg_uri = "sip:$SERVER:$PORT"
    
    acc = lib.create_account(acc_cfg)
    
    # Wait a bit for registration
    import time
    time.sleep(2)
    
    # Check registration status
    if acc.has_registration():
        print("✓ Registration successful!")
        print(f"  Status: {acc.info().reg_status_text}")
    else:
        print("✗ Registration failed or pending")
    
    # Cleanup
    lib.destroy()
    
except Exception as e:
    print(f"Error: {e}")
    sys.exit(1)
EOF
else
    echo "=== Method 4: Python pjsua not available ==="
    echo "Install with: pip3 install pjsua"
    echo ""
fi

echo ""
echo "=== Check FreeSWITCH logs for registration attempts ==="
echo "Run: docker exec -it fs-freeswitch-1 fs_cli -x 'sofia status profile internal reg'"
echo "Or: docker compose logs freeswitch | grep -i register"

