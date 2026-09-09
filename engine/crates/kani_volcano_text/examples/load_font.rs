use anyhow::{Context, Result};
use kani_volcano_text::{TextSystem, GlyphAtlas};

fn main() -> Result<()> {
    let path = std::env::args_os()
        .nth(1)
        .context("usage: load_font <font-path>")?;
    let data = std::fs::read(&path).context("failed to read font file")?;
    let mut system = TextSystem::default();
    let font = system.load_font(data)?;

    let buffer=system.layout_text(
        &font,
        "Hello\nWorld",
        24.0,
        32.0,
    )?;

    let mut atlas=GlyphAtlas::new(1024,1024)?;

    for run in buffer.layout_runs(){
        for glyph in run.glyphs.iter(){
            let physical=glyph.physical((0.0,0.0),1.0);

            let Some(image)=system.rasterize_glyph(physical.cache_key)
            else{
                continue;
            };

            let Some(entry)=atlas.insert(physical.cache_key, image)?
            else{
                continue;
            };

            println!(
                "glyph={}, page={}, pos=({},{}), size={}x{}",
                glyph.glyph_id,
                entry.page,
                entry.x,
                entry.y,
                entry.width,
                entry.height,
            );
        }
    }
    println!("Loaded {} font face(s)", font.face_count());

    Ok(())
}
