use anyhow::{Result};
use tokio::io::{AsyncReadExt, BufReader, AsyncBufReadExt};
use tracing;

use crate::freeswitch::esl::{EslEvent, Headers};

pub struct EslEventFrame {
    pub event_headers: Headers,
    pub event_body: Option<Vec<u8>>
}


pub struct RawFrame {
    pub headers: Headers,
    pub body: Option<Vec<u8>>,
}


pub struct EslParser<R> {
    reader: BufReader<R>
}

impl <R: AsyncReadExt + Unpin> EslParser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader: BufReader::new(reader) }
    }

    pub async fn parse_frame(&mut self) -> Result<Option<EslEvent>> {
        tracing::debug!("[RAW FRAME] Starting to parse raw frame from wire");
        
        let headers = match self.parse_headers().await? {
            Some(h) => {
                tracing::debug!("[RAW FRAME] Parsed {} frame headers: {:?}", h.len(), h);
                h
            },
            None => {
                tracing::debug!("[RAW FRAME] EOF reached, no more frames");
                return Ok(None)
            }
        };

        let body = self.read_body(&headers).await?;
        if let Some(ref body_bytes) = body {
            match String::from_utf8(body_bytes.clone()) {
                Ok(body_str) => {
                    tracing::debug!("[RAW FRAME] Read frame body ({} bytes): {}", body_bytes.len(), body_str);
                }
                Err(_) => {
                    tracing::debug!("[RAW FRAME] Read frame body (binary, {} bytes): {:?}", body_bytes.len(), body_bytes);
                }
            }
        } else {
            tracing::debug!("[RAW FRAME] No body in raw frame");
        }

        let raw_frame = RawFrame {
            headers: headers.clone(),
            body: body.clone(),
        };

        tracing::debug!("[RAW FRAME] Completed raw frame parsing, passing to event parser");
        let event_frame = self.parse_event(raw_frame)?;

        let event = EslEvent {
            frame_headers: headers,
            event_headers: event_frame.event_headers,
            event_body: event_frame.event_body,
        };

        tracing::debug!("[RAW FRAME] Final ESL event constructed - Frame headers: {:?}, Event headers: {:?}, Event body length: {:?}", 
            event.frame_headers,
            event.event_headers,
            event.event_body.as_ref().map(|b| b.len())
        );

        Ok(Some(event))
    }

    pub fn parse_event(&self, raw_frame: RawFrame) -> Result<EslEventFrame> {
        tracing::debug!("[EVENT] Starting to parse event from raw frame");
        
        let content_type = raw_frame.headers.get("Content-Type")
            .cloned()
            .unwrap_or_default();

        tracing::debug!("[EVENT] Content-Type: {}", content_type);

        // If the body contains event data, parse it
        if let Some(body_bytes) = &raw_frame.body {
            match content_type.as_str() {
                "text/event-plain" => {
                    tracing::debug!("[EVENT] Parsing plain text event format");
                    return self.parse_plain_event(body_bytes);
                }
                "text/event-json" | "text/event-xml" => {
                    tracing::debug!("[EVENT] Parsing {} event format (entire body is event body)", content_type);
                    // For JSON/XML, the entire body is the event body
                    // Event headers are empty or could be extracted from the frame headers
                    return Ok(EslEventFrame {
                        event_headers: Headers::new(),
                        event_body: Some(body_bytes.clone()),
                    });
                }
                _ => {
                    tracing::debug!("[EVENT] Unknown content type, using frame headers as event headers");
                    // For other content types, treat the body as event body
                    // and use frame headers as event headers
                    return Ok(EslEventFrame {
                        event_headers: raw_frame.headers.clone(),
                        event_body: Some(body_bytes.clone()),
                    });
                }
            }
        }

        // No body, return empty event frame
        tracing::debug!("[EVENT] No body in raw frame, returning empty event frame");
        Ok(EslEventFrame {
            event_headers: raw_frame.headers.clone(),
            event_body: None,
        })
    }

    fn parse_plain_event(&self, body_bytes: &[u8]) -> Result<EslEventFrame> {
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
                tracing::debug!("[EVENT] Blank line found, finished parsing event headers");
                break;
            }
            
            if let Some((key, value)) = line.trim().split_once(":") {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                tracing::debug!("[EVENT] Parsed event header: {} = {}", key, value);
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
                tracing::debug!("[EVENT] Extracted event body ({} lines)", body_lines.len());
                Some(body_content.into_bytes())
            } else {
                tracing::debug!("[EVENT] No event body after blank line");
                None
            }
        } else {
            tracing::debug!("[EVENT] No blank line found, no event body");
            None
        };

        tracing::debug!("[EVENT] Completed plain event parsing - Event headers: {:?}, Event body length: {:?}", 
            event_headers,
            event_body.as_ref().map(|b| b.len())
        );

        Ok(EslEventFrame {
            event_headers,
            event_body,
        })
    }

    async fn parse_headers(&mut self) -> Result<Option<Headers>> {
        let mut headers = Headers::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if Self::is_eof(bytes_read) {
                tracing::debug!("[RAW FRAME] EOF while parsing frame headers");
                return Ok(None);
            }

            if Self::is_header_break(&line) {
                tracing::debug!("[RAW FRAME] Header break detected, finished parsing frame headers");
                break;
            }

            if let Some((key, value)) = line.trim().split_once(":") {
                let key = key.trim().to_string();
                let value = value.trim().to_string();
                tracing::debug!("[RAW FRAME] Parsed frame header: {} = {}", key, value);
                headers.insert(key, value);
            } else {
                tracing::debug!("[RAW FRAME] Skipping malformed frame header line: {}", line.trim());
            }
        }

        Ok(Some(headers))
    }

    fn is_eof(bytes_read: usize) -> bool {
        bytes_read == 0
    }

    fn is_header_break(line: &str) -> bool {
        line.trim().is_empty()
    }

    async fn read_body(&mut self, headers: &Headers) -> Result<Option<Vec<u8>>> {
        if let Some(content_length_str) = headers.get("Content-Length") {
            tracing::debug!("[RAW FRAME] Found Content-Length header: {}", content_length_str);
            if let Ok(content_length) = content_length_str.parse::<usize>() {
                if content_length > 0 {
                    tracing::debug!("[RAW FRAME] Reading frame body with Content-Length: {}", content_length);
                    let mut body_buf = vec![0u8; content_length];
                    self.reader.read_exact(&mut body_buf).await?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_parse_headers_only() {
        let data = "Content-Type: text/event-plain\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let event = parser.parse_frame().await.unwrap().unwrap();
        
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"0".to_string()));
        assert_eq!(event.event_body, None);
    }

    #[tokio::test]
    async fn test_parse_with_body() {
        let data = "Content-Type: text/event-json\nContent-Length: 15\n\n{\"key\":\"value\"}\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let event = parser.parse_frame().await.unwrap().unwrap();
        
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-json".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"15".to_string()));
        assert_eq!(event.event_body, Some(b"{\"key\":\"value\"}".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_messages() {
        let data = "Content-Type: auth/request\n\nContent-Type: text/event-plain\nContent-Length: 5\n\nhello\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        // Parse first message (no body)
        let event1 = parser.parse_frame().await.unwrap().unwrap();
        assert_eq!(event1.frame_headers.get("Content-Type"), Some(&"auth/request".to_string()));
        assert_eq!(event1.event_body, None);

        // Parse second message (with body)
        let event2 = parser.parse_frame().await.unwrap().unwrap();
        assert_eq!(event2.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(event2.frame_headers.get("Content-Length"), Some(&"5".to_string()));
        assert_eq!(event2.event_body, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_eof() {
        let data = "Content-Type: text/event-plain\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        // Parse the message
        let event = parser.parse_frame().await.unwrap().unwrap();
        assert_eq!(event.frame_headers.get("Content-Type"), Some(&"text/event-plain".to_string()));

        // Next parse should return None (EOF)
        let result = parser.parse_frame().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_body_with_newlines() {
        let data = "Content-Type: text/event-plain\nContent-Length: 12\n\nline1\nline2\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let event = parser.parse_frame().await.unwrap().unwrap();
        assert_eq!(event.event_body, Some(b"line1\nline2".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_headers() {
        let data = "Event-Name: CHANNEL_CREATE\nChannel-State: CS_NEW\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let event = parser.parse_frame().await.unwrap().unwrap();
        assert_eq!(event.frame_headers.get("Event-Name"), Some(&"CHANNEL_CREATE".to_string()));
        assert_eq!(event.frame_headers.get("Channel-State"), Some(&"CS_NEW".to_string()));
        assert_eq!(event.frame_headers.get("Content-Length"), Some(&"0".to_string()));
    }
}