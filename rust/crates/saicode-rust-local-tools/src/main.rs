use std::env;
use std::process;

use saicode_frontline::local_tools;

const FALLBACK_TO_RUST_FULL_CLI_EXIT_CODE: i32 = 90;
const NOT_NATIVE_HANDLED_EXIT_CODE: i32 = 91;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if !local_tools::should_handle_natively(&args) {
        eprintln!("saicode-rust-local-tools cannot handle this invocation natively");
        process::exit(NOT_NATIVE_HANDLED_EXIT_CODE);
    }

    match local_tools::run_native_local_tools(&args) {
        Ok(local_tools::NativeLocalToolsOutcome::Completed) => {}
        Ok(local_tools::NativeLocalToolsOutcome::FallbackToRustFullCli(reason)) => {
            eprintln!("{reason}");
            process::exit(FALLBACK_TO_RUST_FULL_CLI_EXIT_CODE);
        }
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}
