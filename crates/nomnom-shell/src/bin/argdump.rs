//! Test helper: writes the arguments it was started with to the file named by NOMNOM_ARGDUMP.
fn main() {
    if let Some(out) = std::env::var_os("NOMNOM_ARGDUMP") {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let _ = std::fs::write(out, args.join("\n"));
    }
}
