/// Conditionally print messages when not in JSON mode
pub fn log(message: &str, json_output: bool) {
    if !json_output {
        println!("{}", message);
    }
}

/// Conditionally print error messages when not in JSON mode
pub fn error_log(message: &str, json_output: bool) {
    if !json_output {
        eprintln!("{}", message);
    }
}
