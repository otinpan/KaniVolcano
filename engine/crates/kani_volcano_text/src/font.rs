use std::sync::Arc;

use anyhow::{Result, ensure};
use cosmic_text::{CacheKey, SwashImage, FontSystem, fontdb, Buffer, SwashCache};

/// Faces registered from one font file. IDs belong to the owning TextSystem.
#[derive(Debug)]
pub struct LoadedFont {
    // font family have some font face like "Regular", "Bold", "Italic".
    face_ids: Vec<fontdb::ID>,
}

impl LoadedFont {
    pub fn face_count(&self) -> usize {
        self.face_ids.len()
    }
}

/// Shared CPU font database. Only explicitly supplied fonts are loaded.
#[derive(Debug)]
pub struct TextSystem {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            font_system: FontSystem::new_with_locale_and_db(
                "en-US".to_string(),
                fontdb::Database::new(),
            ),
            swash_cache: SwashCache::new(),
        }
    }
}

impl TextSystem {
    /// Takes ownership of font bytes and registers all recognized faces.
    pub fn load_font(&mut self, data: Vec<u8>) -> Result<LoadedFont> {
        let face_ids = self
            .font_system
            .db_mut()
            .load_font_source(fontdb::Source::Binary(Arc::new(data)))
            .to_vec();
        ensure!(
            !face_ids.is_empty(),
            "no supported font faces found in font data"
        );
        Ok(LoadedFont { face_ids })
    }

    pub fn layout_text(
        &mut self,
        font: &LoadedFont,
        text: &str,
        font_size: f32,
        line_height: f32,
    ) -> Result<Buffer>{
        use cosmic_text::{Attrs, Family};
        // use first face
        let face_id=*font.face_ids.first()
            .ok_or_else(||anyhow::anyhow!("font has no face"))?;

        let face=self.font_system.db().face(face_id)
            .ok_or_else(||anyhow::anyhow!("font face not found"))?;

        // the name of font face, that may have some names like "SomeFont", "サムフォント".
        let family=face.families.first()
            .ok_or_else(||anyhow::anyhow!("font has no family name"))?
            .0.clone();

        let attrs=Attrs::new()
            .family(Family::Name(&family))
            .weight(face.weight)
            .style(face.style)
            .stretch(face.stretch);

        crate::layout::layout_text(
            &mut self.font_system,
            text,
            &attrs,
            font_size,
            line_height,
        )
    }


    // create glyph
    pub fn rasterize_glyph(
        &mut self,
        cache_key: CacheKey,
    ) -> Option<&SwashImage>{
        self.swash_cache
            .get_image(&mut self.font_system, cache_key)
            .as_ref()
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_data_does_not_register_faces() {
        let mut system = TextSystem::default();
        for data in [Vec::new(), b"not a font".to_vec()] {
            assert!(system.load_font(data).is_err());
            assert_eq!(system.font_system.db().faces().count(), 0);
        }
    }
}
