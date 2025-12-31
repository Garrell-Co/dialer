use anyhow::{Result};
use tokio::io::{AsyncReadExt, BufReader, AsyncBufReadExt};

use crate::freeswitch::esl::Headers;

pub struct Frame {
    pub content_type: String,
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

    pub async fn parse_event(&mut self) -> Result<Option<Frame>> {
        let headers = match self.parse_headers().await? {
            Some(h) => h,
            None => return Ok(None)
        };

        let body = self.read_body(&headers).await?;

        let content_type = headers.get("Content-Type")
            .cloned()
            .unwrap_or_default();

        Ok(Some(Frame { content_type, headers, body }))
    }

    async fn parse_headers(&mut self) -> Result<Option<Headers>> {
        let mut headers = Headers::new();
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line).await?;

            if Self::is_eof(bytes_read) {
                return Ok(None);
            }

            if Self::is_header_break(&line) {
                break;
            }

            if let Some((key, value)) = line.trim().split_once(":") {
                headers.insert(key.trim().to_string(), value.trim().to_string());
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
            if let Ok(content_length) = content_length_str.parse::<usize>() {
                if content_length > 0 {
                    let mut body_buf = vec![0u8; content_length];
                    self.reader.read_exact(&mut body_buf).await?;
                    return Ok(Some(body_buf));
                }
            }
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

        let frame = parser.parse_event().await.unwrap().unwrap();
        
        assert_eq!(frame.headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(frame.headers.get("Content-Length"), Some(&"0".to_string()));
        assert_eq!(frame.body, None);
    }

    #[tokio::test]
    async fn test_parse_with_body() {
        let data = "Content-Type: text/event-json\nContent-Length: 15\n\n{\"key\":\"value\"}\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let frame = parser.parse_event().await.unwrap().unwrap();
        
        assert_eq!(frame.headers.get("Content-Type"), Some(&"text/event-json".to_string()));
        assert_eq!(frame.headers.get("Content-Length"), Some(&"15".to_string()));
        assert_eq!(frame.body, Some(b"{\"key\":\"value\"}".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_messages() {
        let data = "Content-Type: auth/request\n\nContent-Type: text/event-plain\nContent-Length: 5\n\nhello\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        // Parse first message (no body)
        let frame1 = parser.parse_event().await.unwrap().unwrap();
        assert_eq!(frame1.headers.get("Content-Type"), Some(&"auth/request".to_string()));
        assert_eq!(frame1.body, None);

        // Parse second message (with body)
        let frame2 = parser.parse_event().await.unwrap().unwrap();
        assert_eq!(frame2.headers.get("Content-Type"), Some(&"text/event-plain".to_string()));
        assert_eq!(frame2.headers.get("Content-Length"), Some(&"5".to_string()));
        assert_eq!(frame2.body, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_eof() {
        let data = "Content-Type: text/event-plain\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        // Parse the message
        let frame = parser.parse_event().await.unwrap().unwrap();
        assert_eq!(frame.headers.get("Content-Type"), Some(&"text/event-plain".to_string()));

        // Next parse should return None (EOF)
        let result = parser.parse_event().await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_parse_body_with_newlines() {
        let data = "Content-Type: text/event-plain\nContent-Length: 12\n\nline1\nline2\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let frame = parser.parse_event().await.unwrap().unwrap();
        assert_eq!(frame.body, Some(b"line1\nline2".to_vec()));
    }

    #[tokio::test]
    async fn test_parse_multiple_headers() {
        let data = "Event-Name: CHANNEL_CREATE\nChannel-State: CS_NEW\nContent-Length: 0\n\n";
        let cursor = Cursor::new(data.as_bytes());
        let mut parser = EslParser::new(cursor);

        let frame = parser.parse_event().await.unwrap().unwrap();
        assert_eq!(frame.headers.get("Event-Name"), Some(&"CHANNEL_CREATE".to_string()));
        assert_eq!(frame.headers.get("Channel-State"), Some(&"CS_NEW".to_string()));
        assert_eq!(frame.headers.get("Content-Length"), Some(&"0".to_string()));
    }
}