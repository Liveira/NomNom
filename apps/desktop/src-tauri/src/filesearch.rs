//! Finding files by name for the search box. It asks the system's own index (Windows Search, or
//! `plocate` on Linux), so NomNom keeps no list of files of its own. Only what the system has
//! indexed can be found; on Windows that is the user's folders and the Start menu by default.

use nomnom_core::filesearch::{self as rules, Hit};
use std::time::Duration;

type CmdResult<T> = Result<T, String>;

const LIMIT: u32 = 30;
const TIMEOUT: Duration = Duration::from_secs(5);

/// A PowerShell that stays open for a little while after a search, so the next one answers at once
/// (starting PowerShell takes about two seconds). It reads one query per line and prints one line of
/// JSON back. It closes by itself when nobody searches for a minute and a half.
#[cfg(windows)]
const SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$c = New-Object -ComObject ADODB.Connection
$c.Open("Provider=Search.CollatorDSO;Extended Properties='Application=Windows';")
[Console]::Out.WriteLine('ready'); [Console]::Out.Flush()
while ($true) {
  $sql = [Console]::In.ReadLine()
  if ($sql -eq $null) { break }
  try {
    $r = $c.Execute($sql)
    $rows = New-Object System.Collections.ArrayList
    while (-not $r.EOF) {
      $m = $r.Fields.Item(3).Value
      $when = $null
      if ($m -ne $null -and $m -isnot [System.DBNull]) { $when = [int64]([datetimeoffset]([datetime]$m)).ToUnixTimeSeconds() }
      $s = $r.Fields.Item(2).Value
      if ($s -is [System.DBNull]) { $s = $null }
      [void]$rows.Add([pscustomobject]@{ path = [string]$r.Fields.Item(0).Value; type = [string]$r.Fields.Item(1).Value; size = $s; modified = $when })
      $r.MoveNext()
    }
    if ($rows.Count -eq 0) { $out = '[]' } else { $out = ConvertTo-Json -InputObject @($rows) -Compress }
  } catch { $out = '[]' }
  [Console]::Out.WriteLine($out); [Console]::Out.Flush()
}
"#;

#[cfg(windows)]
struct Worker {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    lines: std::sync::mpsc::Receiver<String>,
    last_used: std::time::Instant,
}

#[cfg(windows)]
static WORKER: std::sync::Mutex<Option<Worker>> = std::sync::Mutex::new(None);

#[cfg(windows)]
const IDLE: Duration = Duration::from_secs(90);

#[cfg(windows)]
fn start_worker() -> CmdResult<Worker> {
    use std::io::{BufRead, BufReader};
    use std::os::windows::process::CommandExt;
    let powershell = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| r"C:\Windows".into())
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let mut child = std::process::Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|e| format!("Couldn't ask Windows Search: {e}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or("Couldn't talk to Windows Search.")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Couldn't talk to Windows Search.")?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    // It says "ready" once it is connected to the index.
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(l) if l.trim() == "ready" => Ok(Worker {
            child,
            stdin,
            lines: rx,
            last_used: std::time::Instant::now(),
        }),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            Err("Windows Search isn't answering.".into())
        }
    }
}

#[cfg(windows)]
fn run_search(term: &str) -> CmdResult<Vec<Hit>> {
    use std::io::Write;
    let sql = rules::windows_sql(term, LIMIT).ok_or("Type at least two letters.")?;
    let mut guard = WORKER.lock().unwrap();
    // A worker that died (or was never there) is made again.
    if guard
        .as_mut()
        .is_some_and(|w| w.child.try_wait().ok().flatten().is_some())
    {
        *guard = None;
    }
    if guard.is_none() {
        *guard = Some(start_worker()?);
    }
    let w = guard.as_mut().expect("just made");
    w.last_used = std::time::Instant::now();
    // The query has no newline in it (the term was cleaned), so it is one line.
    let sent = writeln!(w.stdin, "{sql}").and_then(|_| w.stdin.flush());
    let answer = match sent {
        Ok(()) => w.lines.recv_timeout(TIMEOUT).ok(),
        Err(_) => None,
    };
    match answer {
        Some(line) => Ok(rules::parse_windows(&line)),
        None => {
            // No answer: the worker is stopped, so a late reply can't be taken for the next one.
            if let Some(mut dead) = guard.take() {
                let _ = dead.child.kill();
                let _ = dead.child.wait();
            }
            Err("Windows Search took too long to answer.".into())
        }
    }
}

/// Closes the PowerShell when it has been idle, and when NomNom stops.
#[cfg(windows)]
pub fn spawn_idle_watch() {
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(20));
        let mut guard = WORKER.lock().unwrap();
        if guard.as_ref().is_some_and(|w| w.last_used.elapsed() > IDLE) {
            if let Some(mut w) = guard.take() {
                let _ = w.child.kill();
                let _ = w.child.wait();
            }
        }
    });
}

#[cfg(not(windows))]
fn run_search(term: &str) -> CmdResult<Vec<Hit>> {
    let clean = rules::clean_term(term).ok_or("Type at least two letters.")?;
    let tool = ["plocate", "locate"]
        .into_iter()
        .find(|t| {
            std::process::Command::new(t)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok()
        })
        .ok_or("To find files by name, install plocate.")?;
    let mut cmd = std::process::Command::new(tool);
    cmd.args(["-i", "-A", "-b", "-l", &LIMIT.to_string(), "--"]);
    cmd.args(clean.split(' '));
    let out = cmd.output().map_err(|e| e.to_string())?;
    Ok(rules::parse_locate(
        &String::from_utf8_lossy(&out.stdout),
        LIMIT as usize,
    ))
}

/// Files and folders whose name has every word of the search.
#[tauri::command]
pub async fn fs_search(query: String) -> CmdResult<Vec<Hit>> {
    tokio::task::spawn_blocking(move || run_search(&query))
        .await
        .map_err(|e| e.to_string())?
}
