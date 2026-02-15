use std::io::Read;
use std::time::Instant;
use std::{fs, process};

pub fn run(args: &[String]) {
    let mut file_path = None;
    let mut line_number = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--line" => {
                i += 1;
                line_number = Some(args[i].parse::<usize>().unwrap_or_else(|_| {
                    eprintln!("Error: --line must be a number");
                    process::exit(1);
                }));
            }
            _ => {
                if file_path.is_none() {
                    file_path = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let file_path = file_path.unwrap_or_else(|| {
        eprintln!("Error: missing file path");
        process::exit(1);
    });

    let line_number = line_number.unwrap_or_else(|| {
        eprintln!("Error: missing --line argument");
        process::exit(1);
    });

    let content = fs::read_to_string(&file_path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", file_path, e);
        process::exit(1);
    });

    let block = find_request_block(&content, line_number);
    let request = parse_request(&block);
    execute_request(&request);
}

struct RequestBlock {
    name: Option<String>,
    method: String,
    url: String,
    http_version: Option<String>,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

fn find_request_block(content: &str, target_line: usize) -> String {
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
            return lines[*start..*end].join("\n");
        }
    }

    eprintln!("Error: no request block found at line {}", target_line);
    process::exit(1);
}

fn parse_request(block: &str) -> RequestBlock {
    let mut name: Option<String> = None;
    let mut method = String::new();
    let mut url = String::new();
    let mut http_version: Option<String> = None;
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();
    let mut in_body = false;
    let mut found_request_line = false;

    for line in block.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if found_request_line {
                in_body = true;
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

        if in_body {
            body_lines.push(line);
            continue;
        }

        if !found_request_line {
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                method = parts[0].to_string();
                url = parts[1].to_string();
                if parts.len() == 3 {
                    http_version = Some(parts[2].to_string());
                }
                found_request_line = true;
            }
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }

    if method.is_empty() || url.is_empty() {
        eprintln!("Error: could not parse request (no METHOD URL found)");
        process::exit(1);
    }

    let body = if body_lines.is_empty() {
        None
    } else {
        Some(body_lines.join("\n"))
    };

    RequestBlock {
        name,
        method,
        url,
        http_version,
        headers,
        body,
    }
}

fn execute_request(req: &RequestBlock) {
    let mut request = match req.method.as_str() {
        "GET" => ureq::request("GET", &req.url),
        "POST" => ureq::request("POST", &req.url),
        "PUT" => ureq::request("PUT", &req.url),
        "DELETE" => ureq::request("DELETE", &req.url),
        "PATCH" => ureq::request("PATCH", &req.url),
        "HEAD" => ureq::request("HEAD", &req.url),
        "OPTIONS" => ureq::request("OPTIONS", &req.url),
        other => ureq::request(other, &req.url),
    };

    for (name, value) in &req.headers {
        request = request.set(name, value);
    }

    let start = Instant::now();
    let response = if let Some(body) = &req.body {
        request.send_string(body)
    } else {
        request.call()
    };
    let elapsed = start.elapsed();

    match response {
        Ok(resp) => print_response(resp, req, elapsed),
        Err(ureq::Error::Status(_, resp)) => print_response(resp, req, elapsed),
        Err(ureq::Error::Transport(e)) => {
            eprintln!("Transport error: {}", e);
            process::exit(1);
        }
    }
}

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";

fn status_color(status: u16) -> &'static str {
    match status {
        200..=299 => GREEN,
        300..=399 => YELLOW,
        _ => RED,
    }
}

fn format_duration(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

fn print_response(resp: ureq::Response, req: &RequestBlock, elapsed: std::time::Duration) {
    let status = resp.status();
    let status_text = resp.status_text().to_string();
    let resp_version = resp.http_version().to_string();

    let mut header_lines = Vec::new();
    for name in resp.headers_names() {
        if let Some(value) = resp.header(&name) {
            header_lines.push(format!("{}: {}", name, value));
        }
    }

    let mut body = String::new();
    resp.into_reader().read_to_string(&mut body).ok();
    let body_len = body.len();

    let color = status_color(status);

    let title = match &req.name {
        Some(name) => name.clone(),
        None => format!("{} {}", req.method, req.url),
    };
    print!("\x1b]2;{}\x07", title);

    match &req.http_version {
        Some(ver) => println!("{} {} {}\n", req.method, req.url, ver),
        None => println!("{} {}\n", req.method, req.url),
    }

    println!(
        "{}{} {} {}{}\n",
        color, resp_version, status, status_text, RESET
    );

    for h in &header_lines {
        println!("{}{}{}", DIM, h, RESET);
    }
    println!();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        print!("{}", body);
        if !body.ends_with('\n') {
            println!();
        }
    }

    println!(
        "\n{}{} {} · {} bytes · {}{}",
        color,
        status,
        status_text,
        body_len,
        format_duration(elapsed),
        RESET
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn status_color_2xx_is_green() {
        assert_eq!(status_color(200), GREEN);
        assert_eq!(status_color(201), GREEN);
        assert_eq!(status_color(299), GREEN);
    }

    #[test]
    fn status_color_3xx_is_yellow() {
        assert_eq!(status_color(301), YELLOW);
        assert_eq!(status_color(304), YELLOW);
        assert_eq!(status_color(399), YELLOW);
    }

    #[test]
    fn status_color_4xx_5xx_is_red() {
        assert_eq!(status_color(400), RED);
        assert_eq!(status_color(404), RED);
        assert_eq!(status_color(500), RED);
        assert_eq!(status_color(503), RED);
    }

    #[test]
    fn status_color_1xx_is_red() {
        assert_eq!(status_color(100), RED);
        assert_eq!(status_color(101), RED);
    }

    #[test]
    fn format_duration_under_one_second() {
        assert_eq!(format_duration(Duration::from_millis(0)), "0ms");
        assert_eq!(format_duration(Duration::from_millis(42)), "42ms");
        assert_eq!(format_duration(Duration::from_millis(999)), "999ms");
    }

    #[test]
    fn format_duration_one_second_or_more() {
        assert_eq!(format_duration(Duration::from_millis(1000)), "1.00s");
        assert_eq!(format_duration(Duration::from_millis(1500)), "1.50s");
        assert_eq!(format_duration(Duration::from_millis(2345)), "2.34s");
    }

    #[test]
    fn find_request_block_single_block() {
        let content = "GET https://example.com\nAccept: application/json";
        let block = find_request_block(content, 1);
        assert_eq!(block, "GET https://example.com\nAccept: application/json");
    }

    #[test]
    fn find_request_block_multiple_blocks() {
        let content =
            "GET https://first.com\n###\nPOST https://second.com\n###\nDELETE https://third.com";
        let block = find_request_block(content, 3);
        assert_eq!(block, "POST https://second.com");
    }

    #[test]
    fn find_request_block_last_block_without_trailing_separator() {
        let content = "GET https://first.com\n###\nPOST https://last.com\nContent-Type: text/plain";
        let block = find_request_block(content, 3);
        assert_eq!(block, "POST https://last.com\nContent-Type: text/plain");
    }

    #[test]
    fn find_request_block_with_named_comment() {
        let content = "### My Request\nGET https://example.com";
        let block = find_request_block(content, 1);
        assert_eq!(block, "### My Request\nGET https://example.com");
    }

    #[test]
    fn parse_request_basic_get() {
        let block = "GET https://example.com";
        let req = parse_request(block);
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
        let req = parse_request(block);
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
        let req = parse_request(block);
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.http_version.as_deref(), Some("HTTP/2"));
    }

    #[test]
    fn parse_request_name_from_comment() {
        let block = "### My Request\nGET https://example.com";
        let req = parse_request(block);
        assert_eq!(req.name.as_deref(), Some("My Request"));
    }

    #[test]
    fn parse_request_multiple_headers() {
        let block = "GET https://example.com\nAccept: application/json\nAuthorization: Bearer token123\nX-Custom: value";
        let req = parse_request(block);
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
        let req = parse_request(block);
        assert_eq!(req.method, "DELETE");
        assert_eq!(req.url, "https://example.com/resource/1");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }

    #[test]
    fn parse_request_comments_ignored() {
        let block = "// This is a comment\nGET https://example.com\n// Another comment";
        let req = parse_request(block);
        assert_eq!(req.method, "GET");
        assert_eq!(req.url, "https://example.com");
        assert!(req.headers.is_empty());
    }

    #[test]
    fn parse_request_multiline_body() {
        let block = "POST https://example.com\nContent-Type: application/json\n\n{\n  \"name\": \"test\",\n  \"value\": 42\n}";
        let req = parse_request(block);
        assert_eq!(
            req.body.as_deref(),
            Some("{\n  \"name\": \"test\",\n  \"value\": 42\n}")
        );
    }
}
