use std::collections::HashMap;
use anyhow::{Result,ensure};
use cosmic_text::{SwashImage,CacheKey,SwashContent};
use etagere::{AtlasAllocator,size2};

pub struct GlyphAtlas{
    pages: Vec<AtlasPage>,
    glyphs: HashMap<CacheKey, AtlasGlyph>, 
}


impl GlyphAtlas{
    pub fn new(width: u32, height: u32) -> Result<Self>{
        let page=AtlasPage::new(width,height)?;

        Ok(
            Self { 
                pages: vec![page], 
                glyphs: HashMap::new(), 
            }
        )
    }

    pub fn insert(
        &mut self,
        key: CacheKey,
        image: &SwashImage,
    ) -> Result<Option<AtlasGlyph>>{
        if let Some(glyph) = self.glyphs.get(&key){
            return Ok(Some(*glyph));
        }

        ensure!(
            image.content==SwashContent::Mask,
            "only mask glyph images are supported"
        );

        // get glyph size
        let width=image.placement.width;
        let height=image.placement.height;

        if width==0 || height==0{
            return Ok(None);
        }

        let w=usize::try_from(width)?;
        let h=usize::try_from(height)?;
        let expected_len=w.checked_mul(h)
            .ok_or_else(|| anyhow::anyhow!("glyph image is too large"))?;

        ensure!(
            image.data.len()==expected_len,
            "invalid glyph image data length"
        );

        // glyph's have each 1 pixel empty space
        let padded_width=width.checked_add(2)
            .ok_or_else(||anyhow::anyhow!("glyph width is too large"))?;
        let padded_height=height.checked_add(2)
            .ok_or_else(||anyhow::anyhow!("glyph height is too large"))?;

        
        // get atlas's page size
        let page_width=self.pages[0].width;
        let page_height=self.pages[0].height;

        ensure!(
            padded_width<=page_width&&padded_height<=page_height,
            "glyph does not fit in an atlas page"
        );

        let size=size2(
            i32::try_from(padded_width)?,
            i32::try_from(padded_height)?,
        );

        // search empty page
        let mut allocated=None;
        for (index, page) in self.pages.iter_mut().enumerate(){
            if let Some(allocation) = page.allocator.allocate(size){
                allocated=Some((index, allocation));
                break;
            }
        }
        // if empty page exist, return this. 
        // else create new page
        let (page_index, allocation) = match allocated{
            Some(value) => value,
            None =>{
                let mut page=AtlasPage::new(page_width, page_height)?;

                let allocation=page.allocator.allocate(size)
                    .ok_or_else(||{
                        anyhow::anyhow!("failed to allocate glyph region")
                    })?;

                let index=self.pages.len();
                self.pages.push(page);

                (index,allocation)
            }
        };

        // return (left,top) point of empty space
        let x=allocation.rectangle.min.x as u32+1;
        let y=allocation.rectangle.min.y as u32+1;

        let page=&mut self.pages[page_index];
        let stride=page.width as usize;

        for row in 0..h{
            let src=row*w;
            let dst=(y as usize + row) * stride + x as usize;

            page.pixels[dst..dst + w]
                .copy_from_slice(&image.data[src..src+w]);
        }

        page.dirty=true;

        let glyph=AtlasGlyph{
            page: page_index,
            x,
            y,
            width,
            height,
            left: image.placement.left,
            top: image.placement.top,
        };

        self.glyphs.insert(key, glyph);

        Ok(Some(glyph))
    }
}

// a page of atlas texture
struct AtlasPage{
    allocator: AtlasAllocator,
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    dirty: bool,
}

impl AtlasPage{
    pub fn new(width: u32, height: u32)->Result<Self>{
        ensure!(
            width>0 && height>0,
            "atlas size must be positive"
        );

        let allocator_width=i32::try_from(width)?;
        let allocator_height=i32::try_from(height)?;

        let pixel_count=usize::try_from(width)?
            .checked_mul(usize::try_from(height)?)
            .ok_or_else(||anyhow::anyhow!("atlas size is too large"))?;

        let mut pixels=Vec::new();
        pixels.try_reserve_exact(pixel_count)?;
        pixels.resize(pixel_count,0);

        Ok(Self { 
            allocator: AtlasAllocator::new(size2(
                allocator_width,
                allocator_height,
            )),
            width,
            height,
            pixels,
            dirty: true,
        })
    }
}


#[derive(Clone, Copy, Debug)]
pub struct AtlasGlyph{
    pub page: usize,

    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,

    pub left: i32,
    pub top: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{CacheKeyFlags, fontdb};

    fn key(id: u16) -> CacheKey {
        CacheKey::new(
            fontdb::ID::default(), id, 24.0, (0.0, 0.0),
            fontdb::Weight::NORMAL, CacheKeyFlags::empty(),
        ).0
    }

    fn mask(width: u32, height: u32, data: Vec<u8>) -> SwashImage {
        let mut image = SwashImage::new();
        image.content = SwashContent::Mask;
        image.placement.width = width;
        image.placement.height = height;
        image.placement.left = -2;
        image.placement.top = 7;
        image.data = data;
        image
    }

    fn assert_empty(atlas: &GlyphAtlas) {
        assert_eq!(atlas.pages.len(), 1);
        assert!(atlas.glyphs.is_empty());
        assert!(atlas.pages[0].pixels.iter().all(|&pixel| pixel == 0));
    }

    #[test]
    fn new_creates_a_transparent_dirty_page() -> Result<()> {
        let atlas = GlyphAtlas::new(16, 8)?;
        assert_empty(&atlas);
        let page = &atlas.pages[0];
        assert_eq!((page.width, page.height), (16, 8));
        assert_eq!(page.pixels.len(), 128);
        assert!(page.dirty);
        Ok(())
    }

    #[test]
    fn new_rejects_invalid_dimensions() {
        for (width, height) in [(0, 8), (8, 0), (0, 0), (u32::MAX, 1), (1, u32::MAX)] {
            assert!(GlyphAtlas::new(width, height).is_err());
        }
    }

    #[test]
    fn insert_copies_rows_preserves_padding_and_bearings() -> Result<()> {
        let mut atlas = GlyphAtlas::new(16, 16)?;
        atlas.pages[0].dirty = false;
        let image = mask(3, 2, vec![10, 20, 30, 40, 50, 60]);
        let glyph = atlas.insert(key(1), &image)?.unwrap();
        assert_eq!(glyph.page, 0);
        assert_eq!((glyph.width, glyph.height), (3, 2));
        assert_eq!((glyph.left, glyph.top), (-2, 7));
        let page = &atlas.pages[0];
        assert!(page.dirty);
        assert!(glyph.x >= 1 && glyph.y >= 1);
        assert!(glyph.x + glyph.width < page.width);
        assert!(glyph.y + glyph.height < page.height);
        for y in 0..page.height {
            for x in 0..page.width {
                let expected = if x >= glyph.x && x < glyph.x + glyph.width
                    && y >= glyph.y && y < glyph.y + glyph.height {
                    image.data[((y - glyph.y) * glyph.width + x - glyph.x) as usize]
                } else { 0 };
                assert_eq!(page.pixels[(y * page.width + x) as usize], expected);
            }
        }
        assert_eq!(atlas.glyphs.len(), 1);
        Ok(())
    }

    #[test]
    fn repeated_key_reuses_region_without_dirtying_page() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        let image = mask(6, 6, vec![123; 36]);
        let first = atlas.insert(key(1), &image)?.unwrap();
        let pixels = atlas.pages[0].pixels.clone();
        atlas.pages[0].dirty = false;
        let second = atlas.insert(key(1), &image)?.unwrap();
        assert_eq!((first.page, first.x, first.y), (second.page, second.x, second.y));
        assert_eq!(atlas.pages.len(), 1);
        assert_eq!(atlas.glyphs.len(), 1);
        assert_eq!(atlas.pages[0].pixels, pixels);
        assert!(!atlas.pages[0].dirty);
        Ok(())
    }

    #[test]
    fn distinct_keys_use_nonoverlapping_regions_on_same_page() -> Result<()> {
        let mut atlas = GlyphAtlas::new(32, 32)?;
        let a = atlas.insert(key(1), &mask(2, 2, vec![50; 4]))?.unwrap();
        let b = atlas.insert(key(2), &mask(2, 2, vec![100; 4]))?.unwrap();
        assert_eq!(a.page, b.page);
        assert!(a.x + a.width + 2 <= b.x || b.x + b.width + 2 <= a.x
            || a.y + a.height + 2 <= b.y || b.y + b.height + 2 <= a.y);
        let page = &atlas.pages[a.page];
        assert_eq!(page.pixels[(a.y * page.width + a.x) as usize], 50);
        assert_eq!(page.pixels[(b.y * page.width + b.x) as usize], 100);
        Ok(())
    }

    #[test]
    fn full_page_adds_another_page_without_changing_the_first() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        atlas.insert(key(1), &mask(6, 6, vec![50; 36]))?;
        let original = atlas.pages[0].pixels.clone();
        let second = atlas.insert(key(2), &mask(6, 6, vec![100; 36]))?.unwrap();
        assert_eq!(atlas.pages.len(), 2);
        assert_eq!(second.page, 1);
        assert_eq!(atlas.pages[0].pixels, original);
        assert_eq!((atlas.pages[1].width, atlas.pages[1].height), (8, 8));
        assert!(atlas.pages[1].dirty);
        assert_eq!(atlas.glyphs.len(), 2);
        Ok(())
    }

    #[test]
    fn empty_images_do_not_allocate() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        for (width, height) in [(0, 2), (2, 0), (0, 0)] {
            assert!(atlas.insert(key(1), &mask(width, height, vec![]))?.is_none());
            assert_empty(&atlas);
        }
        Ok(())
    }

    #[test]
    fn invalid_images_leave_atlas_unchanged() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        let mut color = mask(1, 1, vec![255; 4]);
        color.content = SwashContent::Color;
        let mut subpixel = color.clone();
        subpixel.content = SwashContent::SubpixelMask;
        for image in [color, subpixel, mask(2, 2, vec![1; 3]),
            mask(2, 2, vec![1; 5]), mask(7, 1, vec![1; 7]), mask(1, 7, vec![1; 7])] {
            assert!(atlas.insert(key(1), &image).is_err());
            assert_empty(&atlas);
        }
        // A full-page allocation still succeeds after rejected insertions.
        assert!(atlas.insert(key(1), &mask(6, 6, vec![1; 36]))?.is_some());
        assert_eq!(atlas.pages.len(), 1);
        Ok(())
    }
}
