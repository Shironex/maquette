//! Writes the generated test models to a folder, for the mock UI's preview
//! and for dropping into a scratch inbox by hand.
//!
//! ```text
//! cargo run -p maquette-core --example write_fixtures -- <dir>
//! ```

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: write_fixtures <dir>")?,
    );
    std::fs::create_dir_all(&dir)?;
    for (name, bytes) in maquette_core::inbox::fixture::all() {
        let path = dir.join(name);
        std::fs::write(&path, &bytes)?;
        println!("{} ({} bytes)", path.display(), bytes.len());
    }
    Ok(())
}
