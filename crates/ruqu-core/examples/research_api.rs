//! Native stdin/stdout adapter for the shared bounded research JSON API.
use std::io::{self, Read};
fn main() {
    let mut request = String::new();
    let outcome = io::stdin()
        .take((ruqu_core::research_api::MAX_REQUEST_BYTES + 1) as u64)
        .read_to_string(&mut request)
        .map_err(|e| e.to_string())
        .and_then(|_| ruqu_core::research_api::run_json(&request));
    match outcome {
        Ok(result) => println!("{result}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
