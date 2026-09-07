use anyhow::{Context, Result};
use kani_volcano_text::TextSystem;

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .context("usage: load_font <font-path>")?;
    let data = std::fs::read(&path).context("failed to read font file")?;
    let mut system = TextSystem::default();
    let font = system.load_font(data)?;
    println!("Loaded {} font face(s)", font.face_count());
    Ok(())
}
