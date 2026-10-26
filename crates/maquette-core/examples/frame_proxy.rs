//! Runs a frame proxy in front of a dev server on this machine, as the Page
//! view does, for the visual check: prints the proxy's address on the first
//! line, then serves until its stdin closes.
//!
//!   cargo run -q -p maquette-core --example frame_proxy -- http://localhost:3010/

use std::io::{Read as _, Write as _};
use std::process::ExitCode;

use maquette_core::site::preview::FrameProxy;

fn main() -> ExitCode {
    let Some(upstream) = std::env::args().nth(1) else {
        eprintln!("usage: frame_proxy <dev server address>");
        return ExitCode::FAILURE;
    };
    let proxy = match FrameProxy::start(&upstream, 0) {
        Ok(proxy) => proxy,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };
    let mut out = std::io::stdout();
    if writeln!(out, "{}", proxy.url())
        .and_then(|()| out.flush())
        .is_err()
    {
        return ExitCode::FAILURE;
    }
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    drop(proxy);
    ExitCode::SUCCESS
}
