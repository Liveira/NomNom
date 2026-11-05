//! Started by Firefox once per message (`runtime.sendNativeMessage`). Reads one framed request on
//! stdin, writes one framed response on stdout. Nothing else may ever be written to stdout.

use nomnom_ipc::framing::{read_native, write_native};
use nomnom_native_host::{launch_in_background, launchable, read_integration, relay, Options};
use std::io::{stdin, stdout};
use std::path::PathBuf;

fn integration_file() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|d| PathBuf::from(d).join("NOMNOM").join("integration.json"))
}

fn main() {
    let Ok(request) = read_native(&mut stdin().lock()) else {
        return; // Firefox closed the pipe or sent nonsense; there is nobody to answer.
    };
    let integration = integration_file()
        .map(|p| read_integration(&p))
        .unwrap_or_default();
    let mut launch = || launchable(&integration).is_some_and(launch_in_background);
    let answer = relay(
        &request,
        &nomnom_ipc::pipe::pipe_path(),
        &mut launch,
        &Options::default(),
    );
    let _ = write_native(&mut stdout().lock(), &answer);
}
