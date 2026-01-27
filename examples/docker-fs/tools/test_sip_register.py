#!/usr/bin/env python3
"""
Simple SIP REGISTER test script
Tests SIP registration without requiring a full softphone
"""
import socket
import hashlib
import random
import sys
import re

def generate_nonce():
    """Generate a random nonce"""
    return ''.join([format(random.randint(0, 255), '02x') for _ in range(16)])

def md5_hash(data):
    """Calculate MD5 hash"""
    return hashlib.md5(data.encode()).hexdigest()

def send_sip_register(username, password, server, port=5060, realm=None):
    """
    Send a SIP REGISTER request and handle authentication
    """
    if realm is None:
        realm = server
    
    # Create UDP socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.settimeout(5)
    
    try:
        # First REGISTER request (will get 401 Unauthorized)
        call_id = f"{random.randint(100000, 999999)}@{server}"
        cseq = 1
        branch = f"z9hG4bK{random.randint(10000000, 99999999)}"
        tag = f"{random.randint(100000, 999999)}"
        
        register_msg = f"""REGISTER sip:{realm} SIP/2.0\r
Via: SIP/2.0/UDP {server}:{random.randint(5000, 6000)};branch={branch}\r
Max-Forwards: 70\r
To: <sip:{username}@{realm}>\r
From: <sip:{username}@{realm}>;tag={tag}\r
Call-ID: {call_id}\r
CSeq: {cseq} REGISTER\r
Contact: <sip:{username}@{server}:{random.randint(5000, 6000)}>\r
Expires: 3600\r
Content-Length: 0\r
\r
"""
        
        print(f"Sending REGISTER request to {server}:{port}...")
        sock.sendto(register_msg.encode(), (server, port))
        
        # Receive response
        try:
            response, addr = sock.recvfrom(4096)
            response_str = response.decode()
            print(f"Response:\n{response_str}")
            
            # Check for 401 Unauthorized
            if "401" in response_str or "407" in response_str:
                # Extract nonce and realm from response
                nonce_match = re.search(r'nonce="([^"]+)"', response_str)
                realm_match = re.search(r'realm="([^"]+)"', response_str)
                
                if not nonce_match or not realm_match:
                    print("Error: Could not extract nonce/realm from response")
                    return False
                
                nonce = nonce_match.group(1)
                auth_realm = realm_match.group(1)
                
                # Calculate response hash
                ha1 = md5_hash(f"{username}:{auth_realm}:{password}")
                ha2 = md5_hash(f"REGISTER:sip:{auth_realm}")
                response_hash = md5_hash(f"{ha1}:{nonce}:{ha2}")
                
                # Second REGISTER with authentication
                cseq += 1
                branch = f"z9hG4bK{random.randint(10000000, 99999999)}"
                
                auth_header = f'Digest username="{username}", realm="{auth_realm}", nonce="{nonce}", uri="sip:{auth_realm}", response="{response_hash}", algorithm=MD5'
                
                register_msg_auth = f"""REGISTER sip:{auth_realm} SIP/2.0\r
Via: SIP/2.0/UDP {server}:{random.randint(5000, 6000)};branch={branch}\r
Max-Forwards: 70\r
To: <sip:{username}@{auth_realm}>\r
From: <sip:{username}@{auth_realm}>;tag={tag}\r
Call-ID: {call_id}\r
CSeq: {cseq} REGISTER\r
Contact: <sip:{username}@{server}:{random.randint(5000, 6000)}>\r
Authorization: {auth_header}\r
Expires: 3600\r
Content-Length: 0\r
\r
"""
                
                print(f"\nSending authenticated REGISTER request...")
                sock.sendto(register_msg_auth.encode(), (server, port))
                
                # Receive final response
                response, addr = sock.recvfrom(4096)
                response_str = response.decode()
                print(f"Final Response:\n{response_str}")
                
                if "200 OK" in response_str:
                    print("\n✓ Registration successful!")
                    return True
                else:
                    print(f"\n✗ Registration failed: {response_str.split(chr(10))[0]}")
                    return False
            elif "200 OK" in response_str:
                print("\n✓ Registration successful (no auth required)!")
                return True
            else:
                print(f"\n✗ Unexpected response: {response_str.split(chr(10))[0]}")
                return False
                
        except socket.timeout:
            print("Error: Timeout waiting for response")
            return False
            
    except Exception as e:
        print(f"Error: {e}")
        return False
    finally:
        sock.close()

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 test_sip_register.py <username> <password> [server] [port]")
        print("Example: python3 test_sip_register.py 1001 1234 127.0.0.1 5060")
        sys.exit(1)
    
    username = sys.argv[1]
    password = sys.argv[2]
    server = sys.argv[3] if len(sys.argv) > 3 else "127.0.0.1"
    port = int(sys.argv[4]) if len(sys.argv) > 4 else 5060
    
    success = send_sip_register(username, password, server, port)
    sys.exit(0 if success else 1)

