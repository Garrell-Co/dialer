use anyhow::Result; 
use tokio::io::{AsyncReadExt, BufReader, AsyncBufReadExt};
use tracing;
use serde_json::Value;

use crate::freeswitch::esl::{EslEvent, Headers, EslEventFormat};


// The full ESL frame that comes into the TCP connection:
//
// Content-Length: <size>\n                                         | Headers in the TCP/IP
// Content-Type: text/event-plain\n                                 | packet's payload.
// \n
// event-hdr1: a\n       <-- size starts here    | FreeSWITCH       | Event headers and body
// event-hdr2: b\n                               | event headers    | in the the body of the
// ...                                           |                  | TCP/IP packet's body.
// event-hdrN: x\n                               *----------------  |
// \n                    <-- size ends here                         |
// body line 1               (if no body)        *----------------  |
// ...                                           | FreeSWITCH       |
// body line N           <-- or here if there's  | event body       |
//                           a body              |                  |
//
// --------------------------------------------------------------------
// \n is a line feed in the form of CRLF.


pub struct Event {
    pub headers: Headers,
    pub body: Option<Vec<u8>>
}


/// Thin wrapper for owned readers - delegates to standalone functions
pub struct EslReader<R> {
    reader: BufReader<R>,
    event_format: EslEventFormat,
}

impl <R: AsyncReadExt + Unpin> EslReader<R> {
    pub fn new(reader: R, event_format: EslEventFormat) -> Self {
        Self { 
            reader: BufReader::new(reader),
            event_format,
        }
    }

    pub async fn read_next_event(&mut self) -> Result<Option<EslEvent>> {
        tracing::debug!("[RAW FRAME] Waiting to parse raw frame from wire");
        
        let packet_headers = match read_packet_headers_from(&mut self.reader).await? {
            Some(h) => {
                tracing::debug!("[RAW FRAME] Parsed {} frame headers: {:?}", h.len(), h);
                h
            },
            None => {
                tracing::debug!("[RAW FRAME] EOF reached, no more frames");
                return Ok(None)
            }
        };

        let raw_event = read_raw_event_with(&mut self.reader, &packet_headers).await?;
        let ev = self.parse_event(raw_event)?;

        let event = EslEvent {
            frame_headers: packet_headers,
            event_headers: ev.headers,
            event_body: ev.body
        };

        tracing::debug!("[RAW FRAME] Final ESL event constructed - Frame headers: {:?}, Event headers: {:?}, Event body length: {:?}", 
            event.frame_headers,
            event.event_headers,
            event.event_body.as_ref().map(|b| b.len())
        );

        Ok(Some(event))
    }

    /// Parse event from headers and raw body bytes using the configured event format
    fn parse_event(&self, raw_event: Option<Vec<u8>>) -> Result<Event> {
        tracing::debug!("[EVENT] Using event format: {}", self.event_format);

        if let Some(ref body_bytes) = raw_event {
            if let Ok(body_str) = String::from_utf8(body_bytes.clone()) {
                tracing::debug!("[EVENT] Raw event body as UTF-8: {}", body_str);
            } else {
                tracing::debug!("[EVENT] Raw event body is not valid UTF-8 ({} bytes)", body_bytes.len());
            }
        }

        // If the body contains event data, parse it
        if let Some(body_bytes) = &raw_event {
            match self.event_format {
                EslEventFormat::Plain => {
                    tracing::debug!("[EVENT] Parsing plain text event format");
                    return parse_plain_event(body_bytes);
                }
                EslEventFormat::Json => {
                    tracing::debug!("[EVENT] Parsing JSON event format");
                    return parse_json_event(body_bytes);
                },
                EslEventFormat::Xml => {
                    tracing::debug!("[EVENT] Parsing XML event format (entire body is event body)");
                    return Ok(Event {
                        headers: Headers::new(),
                        body: Some(body_bytes.clone()),
                    });
                }
            }
        }

        // No body, return empty event frame
        tracing::debug!("[EVENT] No body in raw frame, returning empty event frame");
        Ok(Event {
            headers: Headers::new(),
            body: None,
        })
    }
}


/// Read packet headers from a borrowed BufReader
pub async fn read_packet_headers_from<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>
) -> Result<Option<Headers>> {
    let mut headers = Headers::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if is_eof(&line, bytes_read) {
            tracing::debug!("[RAW FRAME] EOF while parsing frame headers");
            return Ok(None);
        }

        if is_header_break(&line) {
            tracing::debug!("[RAW FRAME] Header break detected, finished parsing frame headers");
            break;
        }

        if let Some((key, value)) = line.trim().split_once(":") {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            headers.insert(key, value);
        } else {
            tracing::debug!("[RAW FRAME] Skipping malformed frame header line: {}", line.trim());
        }
    }

    Ok(Some(headers))
}

/// Read raw event body from a borrowed BufReader
pub async fn read_raw_event_with<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
    headers: &Headers
) -> Result<Option<Vec<u8>>> {
    if let Some(content_length_str) = headers.get("Content-Length") {
        tracing::debug!("[RAW FRAME] Found Content-Length header: {}", content_length_str);
        if let Ok(content_length) = content_length_str.parse::<usize>() {
            if content_length > 0 {
                tracing::debug!("[RAW FRAME] Reading frame body with Content-Length: {}", content_length);
                let mut body_buf = vec![0u8; content_length];
                reader.read_exact(&mut body_buf).await?;
                tracing::debug!("[RAW FRAME] Successfully read {} bytes of frame body", body_buf.len());
                return Ok(Some(body_buf));
            } else {
                tracing::debug!("[RAW FRAME] Content-Length is 0, no frame body to read");
            }
        } else {
            tracing::warn!("[RAW FRAME] Invalid Content-Length value: {}", content_length_str);
        }
    } else {
        tracing::debug!("[RAW FRAME] No Content-Length header found, no frame body to read");
    }

    Ok(None)
}

/// Parse plain text event format
pub fn parse_plain_event(body_bytes: &[u8]) -> Result<Event> {
    tracing::debug!("[EVENT] Parsing plain text event body ({} bytes)", body_bytes.len());
    let body_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to parse event body as UTF-8: {}", e))?;
    
    let mut lines = body_str.lines();
    let mut event_headers = Headers::new();
    let mut found_blank_line = false;

    // Parse event headers (until blank line)
    for line in lines.by_ref() {
        if line.trim().is_empty() {
            found_blank_line = true;
            break;
        }
        
        if let Some((key, value)) = line.trim().split_once(":") {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            event_headers.insert(key, value);
        } else {
            tracing::debug!("[EVENT] Skipping malformed event header line: {}", line.trim());
        }
    }

    // Extract event body (everything after blank line)
    let event_body = if found_blank_line {
        let body_lines: Vec<&str> = lines.collect();
        if !body_lines.is_empty() {
            let body_content = body_lines.join("\n");
            Some(body_content.into_bytes())
        } else {
            None
        }
    } else {
        None
    };

    tracing::debug!("[EVENT] Completed plain event parsing - Event headers: {:?}, Event body length: {:?}", 
        event_headers,
        event_body.as_ref().map(|b| b.len())
    );

    Ok(Event {
        headers: event_headers,
        body: event_body,
    })
}

/// Parse JSON event format
pub fn parse_json_event(body_bytes: &[u8]) -> Result<Event> {
    tracing::debug!("[EVENT] Parsing JSON event body ({} bytes)", body_bytes.len());
    
    // Convert entire body_bytes to UTF-8
    let body_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to parse JSON event body as UTF-8: {}", e))?;
    
    // Find the first LF (\n) - that's the end of headers (JSON object) and start of body
    // If no newline is found, there's no body - the entire body_bytes is just the JSON object
    // Note: CRLF handling is done at the frame level, so we only see LF here
    let (json_str, event_body) = if let Some(json_end_pos) = body_str.find('\n') {
        // Parse the leading JSON object (everything before the first line feed)
        let json_str = body_str[..json_end_pos].trim();
        
        // Extract remaining data as body (everything after the first line feed)
        // The frame terminator newline is handled by the calling method, so we take
        // everything from after the first newline to the end of body_bytes
        let event_body_str = &body_str[json_end_pos + 1..];
        
        let event_body = if event_body_str.trim().is_empty() {
            None
        } else {
            Some(event_body_str.as_bytes().to_vec())
        };
        
        (json_str, event_body)
    } else {
        // No newline found - entire body_bytes is the JSON object, no body
        (body_str.trim(), None)
    };
    
    // Try to parse JSON, if it fails, return empty headers with body as bytes
    let event_headers = match serde_json::from_str::<Value>(json_str) {
        Ok(json_value) => {
            let mut headers = Headers::new();
            
            // Extract event headers from JSON object
            if let Value::Object(map) = json_value {
                for (key, value) in map {
                    match value {
                        Value::String(s) => {
                            headers.insert(key, s);
                        }
                        Value::Number(n) => {
                            let val_str = n.to_string();
                            headers.insert(key, val_str);
                        }
                        Value::Bool(b) => {
                            let val_str = b.to_string();
                            headers.insert(key, val_str);
                        }
                        Value::Null => {
                            headers.insert(key, "null".to_string());
                        }
                        Value::Array(_) | Value::Object(_) => {
                            // For complex types, serialize to JSON string
                            let val_str = serde_json::to_string(&value)
                                .unwrap_or_else(|_| "{}".to_string());
                            headers.insert(key, val_str);
                        }
                    }
                }
            } else {
                tracing::warn!("[EVENT] JSON value is not an object, using empty headers");
            }
            
            headers
        }
        Err(e) => {
            tracing::warn!("[EVENT] Failed to parse JSON: {}, using empty headers and raw body", e);
            // Return empty headers and use the original body_bytes as the body
            return Ok(Event {
                headers: Headers::new(),
                body: Some(body_bytes.to_vec()),
            });
        }
    };

    Ok(Event {
        headers: event_headers,
        body: event_body,
    })
}

/// Check if line indicates EOF
fn is_eof(line: &str, bytes_read: usize) -> bool {
    bytes_read == 0 || line == "\r\n"
}

/// Check if line is a header break (empty line)
fn is_header_break(line: &str) -> bool {
    line == "\n"
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use crate::freeswitch::esl::EslEventFormat;

    #[tokio::test]
    async fn test_parse_frame_headers_only() {
        let data = "Content-Type: text/event-plain\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        let event = parser.read_next_event().await.unwrap().unwrap();
        
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"0".to_string()));
        assert_eq!(event.event_body, None);
    }

    #[tokio::test]
    async fn test_parse_with_body() {
        let data = "Content-Type: text/event-json\nContent-Length: 15\n\n{\"key\":\"value\"}\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        let event = parser.read_next_event().await.unwrap().unwrap();
        
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-json".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"15".to_string()));
        assert_eq!(event.event_body, Some(b"{\"key\":\"value\"}".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_messages() {
        let data = "Content-Type: auth/request\n\nContent-Type: text/event-plain\nContent-Length: 5\n\nhello\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        // Parse first message (no body)
        let event1 = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(event1.frame_headers.get("Content-Type"), Some(&"auth/request".to_string()));
        assert_eq!(event1.event_body, None);

        // Parse second message (with body)
        let event2 = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(event2.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(event2.frame_headers.get("Content-Length"), Some(&"5".to_string()));
        assert_eq!(event2.event_body, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_eof() {
        let data = "Content-Type: text/event-plain\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        // Parse the message
        let event = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));

        // Next parse should return None (EOF)
        let result = parser.read_next_event().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_body_with_newlines() {
        let data = "Content-Type: text/event-plain\nContent-Length: 12\n\nline1\nline2\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        let event = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(event.event_body, Some(b"line1\nline2".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_headers() {
        let data = "Event-Name: CHANNEL_CREATE\nChannel-State: CS_NEW\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        let event = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(event.frame_headers.get("Event-Name"), Some(&"CHANNEL_CREATE".to_string()));
        assert_eq!(event.frame_headers.get("Channel-State"), Some(&"CS_NEW".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"0".to_string()));
    }

    #[tokio::test]
    async fn test_parse_json_event_heartbeat() {
        // Raw bytes from actual FreeSWITCH HEARTBEAT event
        // The bytes already include a newline (10) at the end after the closing brace (125)
        let full_frame: Vec<u8> = vec![
            123, 34, 69, 118, 101, 110, 116, 45, 78, 97, 109, 101, 34, 58, 34, 72, 69, 65, 82, 84, 66, 69, 65, 84, 34, 44,
            34, 67, 111, 114, 101, 45, 85, 85, 73, 68, 34, 58, 34, 57, 52, 99, 101, 101, 97, 57, 52, 45, 98, 100, 50, 48,
            45, 52, 51, 57, 48, 45, 56, 55, 99, 98, 45, 56, 100, 52, 56, 100, 52, 55, 101, 54, 55, 48, 52, 34, 44,
            34, 70, 114, 101, 101, 83, 87, 73, 84, 67, 72, 45, 72, 111, 115, 116, 110, 97, 109, 101, 34, 58, 34, 101, 52,
            49, 49, 101, 99, 100, 49, 57, 57, 48, 99, 34, 44, 34, 70, 114, 101, 101, 83, 87, 73, 84, 67, 72, 45, 83, 119,
            105, 116, 99, 104, 110, 97, 109, 101, 34, 58, 34, 101, 52, 49, 49, 101, 99, 100, 49, 57, 57, 48, 99, 34, 44,
            34, 70, 114, 101, 101, 83, 87, 73, 84, 67, 72, 45, 73, 80, 118, 52, 34, 58, 34, 49, 55, 50, 46, 49, 55, 46,
            48, 46, 50, 34, 44, 34, 70, 114, 101, 101, 83, 87, 73, 84, 67, 72, 45, 73, 80, 118, 54, 34, 58, 34, 58, 58,
            49, 34, 44, 34, 69, 118, 101, 110, 116, 45, 68, 97, 116, 101, 45, 76, 111, 99, 97, 108, 34, 58, 34, 50, 48,
            50, 54, 45, 48, 49, 45, 48, 54, 32, 49, 52, 58, 51, 54, 58, 53, 50, 34, 44, 34, 69, 118, 101, 110, 116, 45,
            68, 97, 116, 101, 45, 71, 77, 84, 34, 58, 34, 84, 117, 101, 44, 32, 48, 54, 32, 74, 97, 110, 32, 50, 48, 50,
            54, 32, 49, 52, 58, 51, 54, 58, 53, 50, 32, 71, 77, 84, 34, 44, 34, 69, 118, 101, 110, 116, 45, 68, 97, 116,
            101, 45, 84, 105, 109, 101, 115, 116, 97, 109, 112, 34, 58, 34, 49, 55, 54, 55, 55, 49, 48, 50, 49, 50, 52,
            48, 52, 54, 55, 52, 34, 44, 34, 69, 118, 101, 110, 116, 45, 67, 97, 108, 108, 105, 110, 103, 45, 70, 105, 108,
            101, 34, 58, 34, 115, 119, 105, 116, 99, 104, 95, 99, 111, 114, 101, 46, 99, 34, 44, 34, 69, 118, 101, 110,
            116, 45, 67, 97, 108, 108, 105, 110, 103, 45, 70, 117, 110, 99, 116, 105, 111, 110, 34, 58, 34, 115, 101, 110,
            100, 95, 104, 101, 97, 114, 116, 98, 101, 97, 116, 34, 44, 34, 69, 118, 101, 110, 116, 45, 67, 97, 108, 108,
            105, 110, 103, 45, 76, 105, 110, 101, 45, 78, 117, 109, 98, 101, 114, 34, 58, 34, 57, 53, 34, 44, 34, 69, 118,
            101, 110, 116, 45, 83, 101, 113, 117, 101, 110, 99, 101, 34, 58, 34, 53, 55, 54, 34, 44, 34, 69, 118, 101,
            110, 116, 45, 73, 110, 102, 111, 34, 58, 34, 83, 121, 115, 116, 101, 109, 32, 82, 101, 97, 100, 121, 34, 44,
            34, 85, 112, 45, 84, 105, 109, 101, 34, 58, 34, 48, 32, 121, 101, 97, 114, 115, 44, 32, 48, 32, 100, 97, 121,
            115, 44, 32, 48, 32, 104, 111, 117, 114, 115, 44, 32, 48, 32, 109, 105, 110, 117, 116, 101, 115, 44, 32, 49,
            57, 32, 115, 101, 99, 111, 110, 100, 115, 44, 32, 53, 50, 56, 32, 109, 105, 108, 108, 105, 115, 101, 99, 111,
            110, 100, 115, 44, 32, 56, 56, 49, 32, 109, 105, 99, 114, 111, 115, 101, 99, 111, 110, 100, 115, 34, 44, 34,
            70, 114, 101, 101, 83, 87, 73, 84, 67, 72, 45, 86, 101, 114, 115, 105, 111, 110, 34, 58, 34, 49, 46, 49, 48,
            46, 49, 50, 45, 114, 101, 108, 101, 97, 115, 101, 45, 49, 48, 50, 50, 50, 48, 48, 50, 56, 56, 49, 45, 97,
            56, 56, 100, 48, 54, 57, 100, 54, 102, 43, 103, 105, 116, 126, 50, 48, 50, 52, 48, 56, 48, 50, 84, 50, 49, 48,
            50, 50, 55, 90, 126, 97, 56, 56, 100, 48, 54, 57, 100, 54, 102, 126, 54, 52, 98, 105, 116, 34, 44, 34, 85,
            112, 116, 105, 109, 101, 45, 109, 115, 101, 99, 34, 58, 34, 49, 57, 53, 50, 56, 34, 44, 34, 83, 101, 115,
            115, 105, 111, 110, 45, 67, 111, 117, 110, 116, 34, 58, 34, 48, 34, 44, 34, 77, 97, 120, 45, 83, 101, 115,
            115, 105, 111, 110, 115, 34, 58, 34, 49, 48, 48, 48, 34, 44, 34, 83, 101, 115, 115, 105, 111, 110, 45, 80,
            101, 114, 45, 83, 101, 99, 34, 58, 34, 51, 48, 34, 44, 34, 83, 101, 115, 115, 105, 111, 110, 45, 80, 101,
            114, 45, 83, 101, 99, 45, 76, 97, 115, 116, 34, 58, 34, 48, 34, 44, 34, 83, 101, 115, 115, 105, 111, 110,
            45, 80, 101, 114, 45, 83, 101, 99, 45, 77, 97, 120, 34, 58, 34, 48, 34, 44, 34, 83, 101, 115, 115, 105, 111,
            110, 45, 80, 101, 114, 45, 83, 101, 99, 45, 70, 105, 118, 101, 77, 105, 110, 34, 58, 34, 48, 34, 44, 34,
            83, 101, 115, 115, 105, 111, 110, 45, 83, 105, 110, 99, 101, 45, 83, 116, 97, 114, 116, 117, 112, 34, 58,
            34, 48, 34, 44, 34, 83, 101, 115, 115, 105, 111, 110, 45, 80, 101, 97, 107, 45, 77, 97, 120, 34, 58, 34,
            48, 34, 44, 34, 83, 101, 115, 115, 105, 111, 110, 45, 80, 101, 97, 107, 45, 70, 105, 118, 101, 77, 105,
            110, 34, 58, 34, 48, 34, 44, 34, 73, 100, 108, 101, 45, 67, 80, 85, 34, 58, 34, 57, 55, 46, 57, 48, 48,
            48, 48, 48, 34, 125, 10
        ];
        

        let cursor = Cursor::new(full_frame);
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        let event = parser.read_next_event().await.unwrap().unwrap();
        
        // Check frame headers
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-json".to_string()));
        
        // Check event headers extracted from JSON
        assert_eq!(event.event_headers.get("Event-Name"), Some(&"HEARTBEAT".to_string()));
        assert_eq!(event.event_headers.get("Core-UUID"), Some(&"94ceea94-bd20-4390-87cb-8d48d47e6704".to_string()));
        assert_eq!(event.event_headers.get("FreeSWITCH-Hostname"), Some(&"e411ecd1990c".to_string()));
        assert_eq!(event.event_headers.get("Event-Sequence"), Some(&"576".to_string()));
        assert_eq!(event.event_headers.get("Event-Info"), Some(&"System Ready".to_string()));
        
        // Check that body is None (no body content after first newline)
        assert_eq!(event.event_body, None);
    }
}