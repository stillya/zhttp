use crate::error::RunError;

#[derive(Debug)]
pub struct RequestBlock {
    pub name: Option<String>,
    pub method: String,
    pub url: String,
    pub http_version: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

pub fn find_request_block(content: &str, target_line: usize) -> Result<String, RunError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut block_start = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.trim() == "###" {
            if i > block_start {
                blocks.push((block_start, i));
            }
            block_start = i + 1;
        }
    }
    if block_start < lines.len() {
        blocks.push((block_start, lines.len()));
    }

    for (start, end) in &blocks {
        if target_line > *start && target_line <= *end {
            return Ok(lines[*start..*end].join("\n"));
        }
    }

    Err(RunError::NoRequestBlock(target_line))
}

enum ParseState {
    Preamble,
    Headers,
    Body,
}

pub fn parse_request(block: &str) -> Result<RequestBlock, RunError> {
    let mut name: Option<String> = None;
    let mut method = String::new();
    let mut url = String::new();
    let mut http_version: Option<String> = None;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut state = ParseState::Preamble;

    for line in block.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if matches!(state, ParseState::Headers) {
                state = ParseState::Body;
            }
            continue;
        }

        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            if name.is_none() && trimmed.starts_with("###") {
                let comment_text = trimmed.trim_start_matches('#').trim();
                if !comment_text.is_empty() {
                    name = Some(comment_text.to_string());
                }
            }
            continue;
        }

        match state {
            ParseState::Body => {
                body_lines.push(line);
            }
            ParseState::Preamble => {
                let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
                if parts.len() >= 2 {
                    method = parts[0].to_string();
                    url = parts[1].to_string();
                    if parts.len() == 3 {
                        http_version = Some(parts[2].to_string());
                    }
                    state = ParseState::Headers;
                }
            }
            ParseState::Headers => {
                if let Some((key, value)) = trimmed.split_once(':') {
                    headers.push((key.trim().to_string(), value.trim().to_string()));
                }
            }
        }
    }

    if method.is_empty() || url.is_empty() {
        return Err(RunError::ParseFailed("no METHOD URL found".to_string()));
    }

    let body = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    };

    Ok(RequestBlock {
        name,
        method,
        url,
        http_version,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_request_block_single_block() {
        let content = "GET https://example.com\nAccept: application/json";
        let block = find_request_block(content, 1).unwrap();
        assert_eq!(block, "GET https://example.com\nAccept: application/json");
    }

    #[test]
    fn find_request_block_multiple_blocks() {
        let content =
            "GET https://first.com\n###\nPOST https://second.com\n###\nDELETE https://third.com";
        let block = find_request_block(content, 3).unwrap();
        assert_eq!(block, "POST https://second.com");
    }

    #[test]
    fn find_request_block_last_block_without_trailing_separator() {
        let content = "GET https://first.com\n###\nPOST https://last.com\nContent-Type: text/plain";
        let block = find_request_block(content, 3).unwrap();
        assert_eq!(block, "POST https://last.com\nContent-Type: text/plain");
    }

    #[test]
    fn find_request_block_with_named_comment() {
        let content = "### My Request\nGET https://example.com";
        let block = find_request_block(content, 1).unwrap();
        assert_eq!(block, "### My Request\nGET https://example.com");
    }

    #[test]
    fn find_request_block_not_found() {
        let content = "GET https://example.com";
        let err = find_request_block(content, 99).unwrap_err();
        assert!(matches!(err, RunError::NoRequestBlock(99)));
    }

    #[test]
    fn parse_request_basic_get() {
        let block = "GET https://example.com";
        let req = parse_request(block).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.http_version.is_none());
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
        assert!(req.name.is_none());
    }

    #[test]
    fn parse_request_post_with_json_body() {
        let block = "POST https://api.example.com/data\nContent-Type: application/json\n\n{\"key\": \"value\"}";
        let req = parse_request(block).unwrap();
        assert_eq!(req.method, "POST");
        assert_eq!(req.url, "https://api.example.com/data");
        assert_eq!(
            req.headers,
            vec![("Content-Type".to_string(), "application/json".to_string())]
        );
        assert_eq!(req.body.as_deref(), Some("{\"key\": \"value\"}"));
    }

    #[test]
    fn parse_request_with_http_version() {
        let block = "GET https://example.com HTTP/2";
        let req = parse_request(block).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.http_version.as_deref(), Some("HTTP/2"));
    }

    #[test]
    fn parse_request_name_from_comment() {
        let block = "### My Request\nGET https://example.com";
        let req = parse_request(block).unwrap();
        assert_eq!(req.name.as_deref(), Some("My Request"));
    }

    #[test]
    fn parse_request_multiple_headers() {
        let block = "GET https://example.com\nAccept: application/json\nAuthorization: Bearer token123\nX-Custom: value";
        let req = parse_request(block).unwrap();
        assert_eq!(req.headers.len(), 3);
        assert_eq!(
            req.headers[0],
            ("Accept".to_string(), "application/json".to_string())
        );
        assert_eq!(
            req.headers[1],
            ("Authorization".to_string(), "Bearer token123".to_string())
        );
        assert_eq!(
            req.headers[2],
            ("X-Custom".to_string(), "value".to_string())
        );
    }

    #[test]
    fn parse_request_no_headers_no_body() {
        let block = "DELETE https://example.com/resource/1";
        let req = parse_request(block).unwrap();
        assert_eq!(req.method, "DELETE");
        assert_eq!(req.url, "https://example.com/resource/1");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }

    #[test]
    fn parse_request_comments_ignored() {
        let block = "// This is a comment\nGET https://example.com\n// Another comment";
        let req = parse_request(block).unwrap();
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.headers.is_empty());
    }

    #[test]
    fn parse_request_multiline_body() {
        let block = "POST https://example.com\nContent-Type: application/json\n\n{\n  \"name\": \"test\",\n  \"value\": 42\n}";
        let req = parse_request(block).unwrap();
        assert_eq!(
            req.body.as_deref(),
            Some("{\n  \"name\": \"test\",\n  \"value\": 42\n}")
        );
    }

    #[test]
    fn parse_request_empty_block() {
        let err = parse_request("").unwrap_err();
        assert!(matches!(err, RunError::ParseFailed(_)));
    }
}
