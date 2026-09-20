use anyhow::{Result, anyhow};
use cosmic_text::Buffer;

use crate::{GlyphAtlas, TextSystem};

#[derive(Clone, Copy, Debug)]
pub struct TextVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
}

#[derive(Debug)]
pub struct TextBatch {
    pub page: usize,
    pub vertices: Vec<TextVertex>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct TextMesh {
    pub batches: Vec<TextBatch>,
}

pub fn build_text_mesh(
    system: &mut TextSystem,
    atlas: &mut GlyphAtlas,
    buffer: &Buffer,
) -> Result<TextMesh> {
    let mut mesh = TextMesh::default();

    // if buffer's text is not registered in atlas pages,
    // then register it
    // else get its data (page, pos, ...) from AtlasGlyph
    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let key = physical.cache_key;

            let entry = match atlas.get(&key) {
                Some(entry) => entry,
                None => {
                    let Some(image) = system.rasterize_glyph(key) else {
                        continue;
                    };

                    let Some(entry) = atlas.insert(key, image)? else {
                        continue;
                    };

                    entry
                }
            };

            let (page_width, page_height, _, _) = atlas
                .page_data(entry.page)
                .ok_or_else(|| anyhow!("atlas page not found"))?;


            let left = physical.x as f32 + entry.left as f32;
            let top =
                run.line_y + physical.y as f32 - entry.top as f32;
            let right = left + entry.width as f32;
            let bottom = top + entry.height as f32;

            let u0 = entry.x as f32 / page_width as f32;
            let v0 = entry.y as f32 / page_height as f32;
            let u1 =
                (entry.x + entry.width) as f32 / page_width as f32;
            let v1 =
                (entry.y + entry.height) as f32 / page_height as f32;

            // page changed
            // if 'H', 'e' 'l' is page0 and 'o' is page1,
            // mesh have two batches for page0 and page1.
            // each batches include several characters.
            let needs_batch = mesh.batches
                .last()
                .is_none_or(|batch| batch.page != entry.page);

            if needs_batch {
                mesh.batches.push(TextBatch {
                    page: entry.page,
                    vertices: Vec::new(),
                    indices: Vec::new(),
                });
            }

            let batch = mesh.batches.last_mut().unwrap();

            let base = u32::try_from(batch.vertices.len())?;
            anyhow::ensure!(
                base <= u32::MAX - 3,
                "too many text vertices"
            );

            batch.vertices.extend_from_slice(&[
                TextVertex {
                    position: [left, top],
                    uv: [u0, v0],
                },
                TextVertex {
                    position: [right, top],
                    uv: [u1, v0],
                },
                TextVertex {
                    position: [right, bottom],
                    uv: [u1, v1],
                },
                TextVertex {
                    position: [left, bottom],
                    uv: [u0, v1],
                },
            ]);

            batch.indices.extend_from_slice(&[
                base,
                base + 1,
                base + 2,
                base,
                base + 2,
                base + 3,
            ]);
        }
    }

    Ok(mesh)
}
