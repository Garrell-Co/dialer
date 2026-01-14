use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

use crate::freeswitch::esl::{EslEvent, EslEventFormat, Headers};

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
    pub body: Option<Vec<u8>>,
}

/// Thin wrapper for owned readers - delegates to standalone functions
pub struct EslReader<R> {
    reader: BufReader<R>,
    event_format: EslEventFormat,
}

impl<R: AsyncReadExt + Unpin> EslReader<R> {
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
            }
            None => {
                tracing::debug!("[RAW FRAME] EOF reached, no more frames");
                return Ok(None);
            }
        };

        let raw_event = read_raw_event_with(&mut self.reader, &packet_headers).await?;
        let ev = self.parse_event(raw_event)?;

        let event = EslEvent {
            frame_headers: packet_headers,
            event_headers: ev.headers,
            event_body: ev.body,
        };

        tracing::debug!(
            "[RAW FRAME] Final ESL event constructed - Frame headers: {:?}, Event headers: {:?}, Event body length: {:?}, Event body (utf8): {:?}",
            event.frame_headers,
            event.event_headers,
            event.event_body.as_ref().map(|b| b.len()),
            event.event_body.as_ref().and_then(|b| std::str::from_utf8(b).ok())
        );

        Ok(Some(event))
    }

    /// Parse event from headers and raw body bytes using the configured event format
    fn parse_event(&self, raw_event: Option<Vec<u8>>) -> Result<Event> {
        tracing::debug!("[EVENT] Using event format: {}", self.event_format);

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
                }
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
    reader: &mut BufReader<R>,
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
            tracing::debug!(
                "[RAW FRAME] Skipping malformed frame header line: {}",
                line.trim()
            );
        }
    }

    Ok(Some(headers))
}

/// Read raw event body from a borrowed BufReader
pub async fn read_raw_event_with<R: AsyncReadExt + Unpin>(
    reader: &mut BufReader<R>,
    headers: &Headers,
) -> Result<Option<Vec<u8>>> {
    if let Some(content_length_str) = headers.get("Content-Length") {
        tracing::debug!(
            "[RAW FRAME] Found Content-Length header: {}",
            content_length_str
        );
        if let Ok(content_length) = content_length_str.parse::<usize>() {
            if content_length > 0 {
                tracing::debug!(
                    "[RAW FRAME] Reading frame body with Content-Length: {}",
                    content_length
                );
                let mut body_buf = vec![0u8; content_length];
                reader.read_exact(&mut body_buf).await?;
                tracing::debug!(
                    "[RAW FRAME] Successfully read {} bytes of frame body",
                    body_buf.len()
                );
                return Ok(Some(body_buf));
            } else {
                tracing::debug!("[RAW FRAME] Content-Length is 0, no frame body to read");
            }
        } else {
            tracing::warn!(
                "[RAW FRAME] Invalid Content-Length value: {}",
                content_length_str
            );
        }
    } else {
        tracing::debug!("[RAW FRAME] No Content-Length header found, no frame body to read");
    }

    Ok(None)
}

/// Parse plain text event format
pub fn parse_plain_event(body_bytes: &[u8]) -> Result<Event> {
    tracing::debug!(
        "[EVENT] Parsing plain text event body ({} bytes)",
        body_bytes.len()
    );

    // Convert entire body_bytes to UTF-8
    let body_str = String::from_utf8(body_bytes.to_vec())
        .map_err(|e| anyhow::anyhow!("Failed to parse plain text event body as UTF-8: {}", e))?;

    // Find the first LF (\n) - that's the end of headers and start of body
    // If no newline is found, there's no body - the entire body_bytes is just headers (or just body)
    // Note: CRLF handling is done at the frame level, so we only see LF here
    let (headers_str, event_body) = if let Some(headers_end_pos) = body_str.find('\n') {
        // Parse the leading headers (everything before the first line feed)
        let headers_str = body_str[..headers_end_pos].trim();

        // Extract remaining data as body (everything after the first line feed)
        // The frame terminator newline is handled by the calling method, so we take
        // everything from after the first newline to the end of body_bytes
        let event_body_str = &body_str[headers_end_pos + 1..];

        let event_body = if event_body_str.trim().is_empty() {
            None
        } else {
            Some(event_body_str.as_bytes().to_vec())
        };

        (headers_str, event_body)
    } else {
        // No newline found - entire body_bytes is either just headers or just a body
        // Try to parse as headers (key:value pairs), if it doesn't match, treat as body
        let trimmed = body_str.trim();
        (trimmed, None)
    };

    // Parse headers from the headers_str (key:value pairs on separate lines)
    let mut event_headers = Headers::new();
    for line in headers_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue; // Skip blank lines
        }

        if let Some((key, value)) = line.split_once(":") {
            let key = key.trim().to_string();
            let value = value.trim().to_string();
            event_headers.insert(key, value);
        } else {
            tracing::debug!("[EVENT] Skipping malformed event header line: {}", line);
        }
    }

    // If no headers were found and there's no body, treat the entire content as body
    let (final_headers, final_body) = if event_headers.is_empty() && event_body.is_none() {
        (Headers::new(), Some(body_bytes.to_vec()))
    } else {
        (event_headers, event_body)
    };

    tracing::debug!(
        "[EVENT] Completed plain event parsing - Event headers: {:?}, Event body length: {:?}",
        final_headers,
        final_body.as_ref().map(|b| b.len())
    );

    Ok(Event {
        headers: final_headers,
        body: final_body,
    })
}

/// Parse JSON event format
pub fn parse_json_event(body_bytes: &[u8]) -> Result<Event> {
    tracing::debug!(
        "[EVENT] Parsing JSON event body ({} bytes)",
        body_bytes.len()
    );

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
                            let val_str =
                                serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
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
            tracing::debug!(
                "[EVENT] Failed to parse JSON: {}, using empty headers and raw body",
                e
            );
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
    use crate::freeswitch::esl::EslEventFormat;
    use std::{collections::HashMap, io::Cursor};

    #[tokio::test]
    async fn test_parse_frame_headers_only() {
        let data = "Content-Type: text/event-plain\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        let event = parser.read_next_event().await.unwrap().unwrap();

        assert_eq!(
            event.frame_headers.get("Content-Type"),
            Some(&"text/event-plain".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Content-Length"),
            Some(&"0".to_string())
        );
        assert_eq!(event.event_body, None);
    }

    #[tokio::test]
    async fn test_parse_with_only_headers() {
        let data = "Content-Type: text/event-json\nContent-Length: 15\n\n{\"key\":\"value\"}\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        let event = parser.read_next_event().await.unwrap().unwrap();

        assert_eq!(
            event.frame_headers.get("Content-Type"),
            Some(&"text/event-json".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Content-Length"),
            Some(&"15".to_string())
        );
        let mut expected = HashMap::new();
        expected.insert("key".to_string(), "value".to_string());
        assert_eq!(event.event_headers, expected);
    }

    #[tokio::test]
    async fn test_parse_json_with_only_body() {
        let data = "Content-Type: text/event-json\nContent-Length: 18\n\n+OK command result\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        let event = parser.read_next_event().await.unwrap().unwrap();

        assert_eq!(
            event.frame_headers.get("Content-Type"),
            Some(&"text/event-json".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Content-Length"),
            Some(&"18".to_string())
        );
        let expected = HashMap::new();
        assert_eq!(event.event_headers, expected);
        assert_eq!(event.event_body, Some(b"+OK command result".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_json_with_headers_and_body() {
        let data = "Content-Type: text/event-json\nContent-Length: 34\n\n{\"key\":\"value\"}\n+OK command result\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        let event = parser.read_next_event().await.unwrap().unwrap();

        assert_eq!(
            event.frame_headers.get("Content-Type"),
            Some(&"text/event-json".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Content-Length"),
            Some(&"34".to_string())
        );
        let mut expected = HashMap::new();
        expected.insert("key".to_string(), "value".to_string());
        assert_eq!(event.event_headers, expected);
        assert_eq!(event.event_body, Some(b"+OK command result".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_messages() {
        let data = "Content-Type: auth/request\n\nContent-Type: text/event-json\nContent-Length: 5\n\nhello\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Json);

        // Parse first message (no body)
        let event1 = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(
            event1.frame_headers.get("Content-Type"),
            Some(&"auth/request".to_string())
        );
        assert_eq!(event1.event_body, None);

        // Parse second message (with body)
        let event2 = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(
            event2.frame_headers.get("Content-Type"),
            Some(&"text/event-json".to_string())
        );
        assert_eq!(
            event2.frame_headers.get("Content-Length"),
            Some(&"5".to_string())
        );
        assert_eq!(event2.event_body, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_eof() {
        let data = "Content-Type: text/event-plain\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        // Parse the message
        let event = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(
            event.frame_headers.get("Content-Type"),
            Some(&"text/event-plain".to_string())
        );

        // Next parse should return None (EOF)
        let result = parser.read_next_event().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_multiple_headers() {
        let data = "Event-Name: CHANNEL_CREATE\nChannel-State: CS_NEW\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslReader::new(cursor, EslEventFormat::Plain);

        let event = parser.read_next_event().await.unwrap().unwrap();
        assert_eq!(
            event.frame_headers.get("Event-Name"),
            Some(&"CHANNEL_CREATE".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Channel-State"),
            Some(&"CS_NEW".to_string())
        );
        assert_eq!(
            event.frame_headers.get("Content-Length"),
            Some(&"0".to_string())
        );
    }
}
