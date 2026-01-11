#!/usr/bin/env python3
import socket
import json
import time
import sys
import os
from datetime import datetime

# Config
ESL_HOST = "127.0.0.1"
ESL_PORT = 8021
ESL_PASSWORD = "ClueCon"
LOG_FILE = "../data/call_lifecycle.jsonl"

def connect():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.connect((ESL_HOST, ESL_PORT))
        return s
    except Exception as e:
        print(f"Error connecting to ESL: {e}")
        return None

def send(s, cmd):
    s.send((cmd + "\n\n").encode('utf-8'))

def read_packet(s):
    headers = {}
    content = ""
    while True:
        line = ""
        while True:
            char = s.recv(1)
            if not char:
                return None, None
            line += char.decode('utf-8')
            if line.endswith("\n"):
                break
        
        line = line.strip()
        if not line:
            # End of headers
            break
        
        if ":" in line:
            k, v = line.split(":", 1)
            headers[k.strip()] = v.strip()
    
    if "Content-Length" in headers:
        length = int(headers["Content-Length"])
        content = s.recv(length).decode('utf-8')
    
    return headers, content

def main():
    print(f"Starting ESL Logger. Connecting to {ESL_HOST}:{ESL_PORT}...")
    
    # Ensure data dir exists
    log_dir = os.path.dirname(LOG_FILE)
    if log_dir and not os.path.exists(log_dir):
        os.makedirs(log_dir)

    while True:
        s = connect()
        if not s:
            time.sleep(5)
            continue
            
        try:
            # Auth
            headers, _ = read_packet(s)
            if headers.get("Content-Type") != "auth/request":
                print("Did not receive auth request.")
                s.close()
                time.sleep(5)
                continue
            
            send(s, f"auth {ESL_PASSWORD}")
            headers, _ = read_packet(s)
            if headers.get("Reply-Text") != "+OK accepted":
                print("Auth failed.")
                s.close()
                return

            print("Authenticated. Subscribing to events...")
            send(s, "events json CHANNEL_CREATE CHANNEL_PROGRESS CHANNEL_ANSWER CHANNEL_HANGUP_COMPLETE")
            
            # Consume command reply for events
            read_packet(s)

            print(f"Listening for calls. Logging to {LOG_FILE}")
            
            with open(LOG_FILE, "a") as f:
                while True:
                    headers, content = read_packet(s)
                    if not headers:
                        break
                        
                    if headers.get("Content-Type") == "text/event-json" and content:
                        try:
                            event = json.loads(content)
                            event_name = event.get("Event-Name")
                            uuid = event.get("Unique-ID")
                            timestamp = datetime.now().isoformat()
                            did = event.get("variable_did", "unknown")
                            agent = event.get("variable_agent_ext", "unknown")
                            
                            log_entry = {
                                "timestamp": timestamp,
                                "uuid": uuid,
                                "event": event_name,
                                "did": did,
                                "agent": agent,
                                "state": event.get("Channel-State")
                            }
                            
                            line = json.dumps(log_entry)
                            f.write(line + "\n")
                            f.flush()
                            print(f"[{timestamp}] {event_name} UUID={uuid} DID={did} Agent={agent}")
                            
                        except json.JSONDecodeError:
                            pass
                            
        except Exception as e:
            print(f"Connection lost: {e}")
            if s:
                s.close()
            time.sleep(3)

if __name__ == "__main__":
    main()
