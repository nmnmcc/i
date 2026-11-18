//! Standard output whose reader may go away early, as when `i spec | head`
//! reads a few lines and exits.
//!
//! Writing to a pipe that no one reads gives `BrokenPipe`. That is not a
//! failure of the command: the reader has seen what it asked for, so the
//! command stops quietly with success, as `cat | head` does.

use std::io::{self, Write};

/// The error of a write to standard output whose reader has gone. Only this
/// marks a closed pipe: a `BrokenPipe` from any other write, such as to a
/// child process, is a real failure.
#[derive(Debug)]
pub(crate) struct Closed;

impl std::fmt::Display for Closed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("standard output's reader went away")
    }
}

impl std::error::Error for Closed {}

/// Writes `text` to standard output, leaving the error to the caller, which
/// recognises a closed pipe with [`is_broken_pipe`].
pub(crate) fn write(text: &str) -> anyhow::Result<()> {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Err(Closed.into()),
        Err(error) => Err(error.into()),
    }
}

/// Whether `error` came from writing to a standard output whose reader has
/// gone, as reported by [`write`].
pub(crate) fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Closed>().is_some()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_pipe_from_another_write_is_a_failure() {
        let error = anyhow::Error::new(io::Error::from(io::ErrorKind::BrokenPipe));
        assert!(!is_broken_pipe(&error));
        assert!(!is_broken_pipe(&error.context("writing to `tar`")));
    }

    #[test]
    fn a_closed_standard_output_is_not_a_failure() {
        assert!(is_broken_pipe(&anyhow::Error::new(Closed)));
        assert!(is_broken_pipe(
            &anyhow::Error::new(Closed).context("i spec")
        ));
    }
}
