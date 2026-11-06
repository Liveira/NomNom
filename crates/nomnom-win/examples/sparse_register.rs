//! Development helper: tries to register (or remove) the menu package.
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("add") if a.len() == 4 => println!(
            "{:?}",
            nomnom_win::sparse::register(std::path::Path::new(&a[2]), std::path::Path::new(&a[3]))
        ),
        Some("remove") => println!("{:?}", nomnom_win::sparse::unregister()),
        Some("list") => println!("{:?}", nomnom_win::sparse::installed()),
        _ => eprintln!("usage: sparse_register add <msix> <install dir> | remove | list"),
    }
}
