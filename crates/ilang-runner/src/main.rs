//! The runner of native packages (docs/design/native.md, N2). `i build`
//! appends a compiled program to a copy of this executable; it runs that
//! program with the arguments, environment and standard streams it is given,
//! and exits as `i run` does. See `ilang_host::native`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            eprintln!("cannot find this executable: {error}");
            return ExitCode::FAILURE;
        }
    };
    ilang_host::native::run(&exe, std::env::args().skip(1).collect())
}
