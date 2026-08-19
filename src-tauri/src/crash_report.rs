//! Crash reporter — installed as a panic hook in both `monoclip` (GUI) and
//! `mclip` (CLI sidecar) `main()`. Never panics itself and never blocks
//! longer than a few seconds; it's an observability side effect, not
//! something that should change crash behavior for the user.
//!
//! Shells out to `monomind report-crash` (from the sibling monomind CLI) so
//! redaction, dedup against existing GitHub issues, and auth (gh CLI /
//! GITHUB_TOKEN) logic live in one place instead of being reimplemented here.
//! If monomind isn't installed, the crash is saved to a local log file instead.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// `binary_name` distinguishes crashes from `monoclip` (GUI) vs `mclip` (CLI)
/// in the report title, since both share this one panic-report path.
pub fn install(binary_name: &'static str) {
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("{panic_info}");

        let title = format!("panic in {binary_name}: {panic_info}");
        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let body = format!(
            "Uncaught panic in `{}` v{}.\n\n```\n{}\n```\n",
            binary_name,
            env!("CARGO_PKG_VERSION"),
            backtrace
        );
        report_crash(&title, &body);
    }));
}

fn report_crash(title: &str, body: &str) {
    if which("monomind").is_some() && run_report("monomind", &[], title, body) {
        return;
    }
    if which("npx").is_some() && run_report("npx", &["-y", "monomind"], title, body) {
        return;
    }
    save_locally(title, body);
}

/// Minimal PATH lookup — avoids spawning a process just to check existence.
fn which(bin: &str) -> Option<()> {
    let path_var = std::env::var_os("PATH")?;
    std::env::split_paths(&path_var).find_map(|dir| {
        let candidate = dir.join(bin);
        candidate.is_file().then_some(())
    })
}

/// Returns false if the invocation failed (e.g. an older monomind on PATH
/// predates the `report-crash` command) so the caller can fall back.
fn run_report(program: &str, prefix_args: &[&str], title: &str, body: &str) -> bool {
    let mut cmd = Command::new(program);
    cmd.args(prefix_args)
        .arg("report-crash")
        .arg("--repo")
        .arg("monoes/mono-clip")
        .arg("--title")
        .arg(title)
        .arg("--body")
        .arg(body)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let timeout = Duration::from_secs(20);
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return false,
        }
    }
}

fn save_locally(title: &str, body: &str) {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) else {
        eprintln!("[monoclip] crash occurred, and couldn't save a report (no HOME): {title}");
        return;
    };
    let crash_dir = std::path::Path::new(&home).join(".monoclip/crashes");
    if std::fs::create_dir_all(&crash_dir).is_err() {
        return;
    }
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = crash_dir.join(format!("{ts}.md"));
    if let Ok(mut f) = std::fs::File::create(&path) {
        let _ = write!(f, "# {title}\n\n{body}\n");
        eprintln!(
            "[monoclip] crash report saved to {} (install monomind to auto-file: npm i -g monomind)",
            path.display()
        );
    }
}
