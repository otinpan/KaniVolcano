use std::sync::Arc;

use anyhow::{Result, ensure};
use cosmic_text::{FontSystem, fontdb};

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
}

impl Default for TextSystem {
    fn default() -> Self {
        Self {
            font_system: FontSystem::new_with_locale_and_db(
                "en-US".to_string(),
                fontdb::Database::new(),
            ),
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
