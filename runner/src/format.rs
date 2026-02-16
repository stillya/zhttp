use std::io::Read;
use std::time::Duration;

use crate::parse::RequestBlock;

pub const RESET: &str = "\x1b[0m";
pub const DIM: &str = "\x1b[2m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";

pub fn status_color(status: u16) -> &'static str {
    match status {
        200..=299 => GREEN,
        300..=399 => YELLOW,
        _ => RED,
    }
}

pub fn format_duration(d: Duration) -> String {
    let ms = d.as_millis();
    if ms < 1000 {
        format!("{}ms", ms)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

pub fn print_response(resp: ureq::Response, req: &RequestBlock, elapsed: Duration) {
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
}
