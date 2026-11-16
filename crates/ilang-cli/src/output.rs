//! Standard output whose reader may go away early, as when `i spec | head`
//! reads a few lines and exits.
//!
//! Writing to a pipe that no one reads gives `BrokenPipe`. That is not a
//! failure of the command: the reader has seen what it asked for, so the
//! command stops quietly with success, as `cat | head` does.

use std::io::{self, Write};

/// Writes `text` to standard output, leaving the error to the caller, which
/// recognises a closed pipe with [`is_broken_pipe`].
pub(crate) fn write(text: &str) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

/// Whether `error` was caused by writing to a pipe whose reader has gone.
pub(crate) fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::BrokenPipe)
    })
}

/// Ends the process after a failed write to standard output: quietly and
/// with success for a closed pipe, which is how command-line tools take
/// `… | head`; other write errors keep the usual failure.
pub(crate) fn exit_after_write(error: &io::Error) -> ! {
    if error.kind() == io::ErrorKind::BrokenPipe {
        std::process::exit(0);
    }
    eprintln!("error writing standard output: {error}");
    std::process::exit(1);
}

/// `println!`, except that a closed pipe ends the process at once instead of
/// panicking: `i test | head` stops when the reader has seen enough.
macro_rules! outln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        if let Err(error) = writeln!(std::io::stdout(), $($arg)*) {
            $crate::output::exit_after_write(&error);
        }
    }};
}
pub(crate) use outln;
