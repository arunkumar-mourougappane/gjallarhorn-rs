//! # Gjallarhorn Binary
//!
//! Entry point for the executable. Delegates code to the library.

fn main() -> Result<(), slint::PlatformError> {
    // Check for worker flag
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--privileged-worker".to_string()) {
        gjallarhorn::worker::run_worker();
        return Ok(());
    }

    gjallarhorn::run()
}
