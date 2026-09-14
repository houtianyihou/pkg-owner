use std::os::unix::process::CommandExt;
use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn run(args: &[&str]) -> Result<String, String> {
    run_timeout(args, Duration::from_secs(40))
}
pub fn run_timeout(args: &[&str], timeout: Duration) -> Result<String, String> {
    let mut child = Command::new(args[0])
        .args(&args[1..])
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_ANALYTICS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = thread::spawn(move || {
        let mut b = Vec::new();
        stdout.read_to_end(&mut b).map(|_| b)
    });
    let err = thread::spawn(move || {
        let mut b = Vec::new();
        stderr.read_to_end(&mut b).map(|_| b)
    });
    let start = Instant::now();
    let mut status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) if start.elapsed() < timeout => thread::sleep(Duration::from_millis(20)),
            result => {
                // Kill the command group, including descendants holding output pipes.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                break Err(match result {
                    Err(e) => e.to_string(),
                    _ => "command timed out".into(),
                });
            }
        }
    };
    // A child may exit while a descendant still holds stdout/stderr open.
    // Keep the same deadline while draining output rather than blocking on join.
    while !out.is_finished() || !err.is_finished() {
        if start.elapsed() >= timeout {
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            status = Err("command timed out".into());
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let stdout = out
        .join()
        .map_err(|_| "stdout reader panicked")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "stderr reader panicked")?
        .map_err(|e| e.to_string())?;
    let status = status?;
    if !status.success() {
        let detail: String = String::from_utf8_lossy(&stderr)
            .trim()
            .chars()
            .take(180)
            .collect();
        return Err(if detail.is_empty() {
            format!("exit {status}")
        } else {
            detail
        });
    }
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn output_error_timeout() {
        assert_eq!(run(&["/bin/echo", "hello"]).unwrap(), "hello\n");
        assert!(run(&["/bin/sh", "-c", "echo failure >&2; exit 3"])
            .unwrap_err()
            .contains("failure"));
        let start = Instant::now();
        assert!(
            run_timeout(&["/bin/sh", "-c", "sleep 10"], Duration::from_millis(50))
                .unwrap_err()
                .contains("timed out")
        );
        assert!(start.elapsed() < Duration::from_secs(3));
        let start = Instant::now();
        assert!(run_timeout(
            &["/bin/sh", "-c", "sleep 10 & exit 0"],
            Duration::from_millis(50)
        )
        .is_err());
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
