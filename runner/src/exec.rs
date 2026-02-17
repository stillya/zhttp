use std::time::Instant;

use crate::error::RunError;
use crate::format::print_response;
use crate::parse::RequestBlock;

pub fn execute_request(req: &RequestBlock) -> Result<(), RunError> {
    let mut request = ureq::request(&req.method, &req.url);

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
        Ok(resp) => {
            print_response(resp, req, elapsed);
            Ok(())
        }
        Err(ureq::Error::Status(_, resp)) => {
            print_response(resp, req, elapsed);
            Ok(())
        }
        Err(ureq::Error::Transport(e)) => Err(RunError::Transport(e.to_string())),
    }
}
