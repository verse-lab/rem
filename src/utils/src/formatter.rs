use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::error::Error;

/// Formats a rust source string using rustfmt.
pub fn format_source(src: &str) -> Result<String, Error> {
    let rustfmt = {
        let mut proc = Command::new(&"rustfmt")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let mut stdin = proc.stdin.take().unwrap();
        stdin.write_all(src.as_bytes())?;
        proc
    };

    let stdout = rustfmt.wait_with_output()?;

    let src = String::from_utf8(stdout.stdout)?;

    Ok(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_source_works() {
        let src = format_source("pub fn add1(x : i32) { x + 1}").unwrap();

        assert!(!src.is_empty());
    }
}
