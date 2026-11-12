//! Times one read-only scan of a folder or drive with the same settings the app uses.
//!
//!     cargo run --release -p nomnom-storage --example scan_bench -- C:\ [workers] [hold_seconds]
//!
//! It only reads. `hold_seconds` keeps the process alive after the scan so its memory can be
//! sampled from outside (Task Manager, `Get-Process`).

use nomnom_storage::{drives, ScanOptions, Session};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| r"C:\".into()));
    let workers: Option<usize> = args.next().and_then(|s| s.parse().ok());
    let hold: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let drive = drives::drive_of(&root);
    let opts = ScanOptions {
        workers: workers.unwrap_or(4),
        cluster: drive.as_ref().map(|d| d.cluster).unwrap_or(4096),
        volume_serial: drive.as_ref().map(|d| d.serial).unwrap_or(0),
        ..ScanOptions::default()
    };
    println!("scanning {} with {} workers", root.display(), opts.workers);
    let t = Instant::now();
    let session = Session::start(1, root, opts);
    while !session.is_finished() {
        std::thread::sleep(Duration::from_millis(100));
    }
    let wall = t.elapsed();
    let idx = session.index.read().unwrap();
    let s = &session.stats;
    use std::sync::atomic::Ordering::SeqCst;
    println!("state      {:?}", session.state());
    println!("wall time  {:.2} s", wall.as_secs_f64());
    println!("files      {}", s.files.load(SeqCst));
    println!("folders    {}", s.dirs.load(SeqCst));
    println!("bytes      {}", s.bytes.load(SeqCst));
    println!("errors     {}", s.errors.load(SeqCst));
    println!("nodes      {}", idx.len());
    if let Some(r) = idx.node(idx.root()) {
        println!("root size  {}", r.size);
        println!("root alloc {}", r.alloc);
    }
    if let Some(d) = &drive {
        println!("drive used {} of {}", d.used, d.total);
    }
    // What the other tabs cost on this many entries.
    use nomnom_storage::analyze::{by_category, by_extension, top_files, Filter};
    use nomnom_storage::search::{compile, run, Query, TextMode};
    let time = |label: &str, f: &dyn Fn() -> String| {
        let t = Instant::now();
        let what = f();
        println!(
            "{label:<34} {:>7.0} ms  {what}",
            t.elapsed().as_secs_f64() * 1000.0
        );
    };
    time("top 100 files", &|| {
        format!("{} rows", top_files(&idx, 0, 100, &Filter::default()).len())
    });
    time("extensions and categories", &|| {
        format!(
            "{} + {}",
            by_extension(&idx, 0, 200).len(),
            by_category(&idx, 0).len()
        )
    });
    let never = std::sync::atomic::AtomicBool::new(false);
    let query = |q: Query| {
        let c = compile(&q, 0).expect("query compiles");
        let f = run(&idx, 0, &c, 500, &never);
        format!("{} found, {} shown", f.total, f.items.len())
    };
    time("search name *.dll", &|| {
        query(Query::Name {
            text: "*.dll".into(),
            mode: TextMode::Wildcard,
        })
    });
    time("search path contains cache", &|| {
        query(Query::Path {
            text: "cache".into(),
            mode: TextMode::Contains,
        })
    });
    drop(idx);
    if hold > 0 {
        println!("holding {hold} s");
        std::thread::sleep(Duration::from_secs(hold));
    }
}
