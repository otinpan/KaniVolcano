use anyhow::{Result,anyhow};

use crate::image::{create_mask_texture_from_pixels, update_mask_texture_from_pixels};
use crate::types::{Texture,VulkanData};
use crate::{Instance, Device, MeshHandle};
use kani_volcano_text::GlyphAtlas;
use vulkanalia::vk::DeviceV1_0;

#[derive(Debug)]
pub struct GpuTextBatch{
    pub page: usize,
    pub mesh: MeshHandle,
}

#[derive(Debug, Default)]
pub struct GpuTextMesh{
    pub batches: Vec<GpuTextBatch>,
}

// record gpu memory for texture
// create new page, store and send to gpu
// update dirty page
// release memory
pub(crate) struct GpuGlyphAtlas{
    textures: Vec<Option<Texture>>,
}

impl GpuGlyphAtlas{
    pub fn new() -> Self{
        Self{
            textures: Vec::new(),
        }
    }

    pub unsafe fn upload_dirty_pages(
        &mut self,
        instance: &Instance,
        device: &Device,
        data: &mut VulkanData,
        atlas: &mut GlyphAtlas,
    ) -> Result<()>{
        self.upload_dirty_pages_with(atlas, |existing, pixels, width, height| {
            match existing {
                None => Ok(Some(create_mask_texture_from_pixels(
                    instance, device, data, pixels, width, height,
                )?)),
                Some(texture) => {
                    update_mask_texture_from_pixels(
                        instance, device, data, texture, pixels, width, height,
                    )?;
                    Ok(None)
                }
            }
        })
    }

    // Keep page bookkeeping testable without creating a Vulkan device.
    fn upload_dirty_pages_with(
        &mut self,
        atlas: &mut GlyphAtlas,
        mut upload: impl FnMut(Option<&Texture>, &[u8], u32, u32) -> Result<Option<Texture>>,
    ) -> Result<()> {
        self.textures.resize_with(
            atlas.page_count(),
            || None,
        );

        for index in 0..atlas.page_count(){
            let (width, height, pixels, dirty)=atlas
                .page_data(index)
                .ok_or_else(|| anyhow!("atlas page not found"))?;

            let slot=&mut self.textures[index];

            match slot.as_ref(){
                None=>{
                    let texture = upload(None, pixels, width, height)?
                        .ok_or_else(|| anyhow!("texture creation returned no texture"))?;

                    *slot=Some(texture);
                }
                Some(texture) if dirty =>{
                    upload(Some(texture), pixels, width, height)?;
                }
                Some(_) =>continue,
            }

            // if successed, make dirty be false
            atlas.mark_page_uploaded(index);
        }
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        for texture in self.textures.drain(..).flatten() {
            device.destroy_image_view(texture.image_view, None);
            device.destroy_image(texture.image, None);
            device.free_memory(texture.image_memory, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic_text::{CacheKey, CacheKeyFlags, SwashImage, fontdb};
    use vulkanalia::vk::{self, Handle};

    // These handles are only compared; no Vulkan operation receives them.
    fn texture(id: u64) -> Texture {
        Texture {
            image: vk::Image::from_raw(id),
            image_memory: vk::DeviceMemory::null(),
            image_view: vk::ImageView::null(),
            mip_levels: 1,
        }
    }

    fn insert_full_page(atlas: &mut GlyphAtlas, id: u16) -> Result<()> {
        let key = CacheKey::new(fontdb::ID::default(), id, 24.0, (0.0, 0.0),
            fontdb::Weight::NORMAL, CacheKeyFlags::empty()).0;
        let mut image = SwashImage::new();
        image.placement.width = 6;
        image.placement.height = 6;
        image.data = vec![id as u8; 36];
        atlas.insert(key, &image)?;
        Ok(())
    }

    #[test]
    fn new_has_no_gpu_textures() {
        assert!(GpuGlyphAtlas::new().textures.is_empty());
    }

    #[test]
    fn missing_texture_is_created_even_when_cpu_page_is_clean() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        atlas.mark_page_uploaded(0);
        let mut gpu = GpuGlyphAtlas::new();
        let mut calls = 0;
        gpu.upload_dirty_pages_with(&mut atlas, |existing, pixels, width, height| {
            calls += 1;
            assert!(existing.is_none());
            assert_eq!((width, height), (8, 8));
            assert_eq!(pixels, &[0; 64]);
            Ok(Some(texture(1)))
        })?;
        assert_eq!(calls, 1);
        assert_eq!(gpu.textures, vec![Some(texture(1))]);
        assert!(!atlas.page_data(0).unwrap().3);
        Ok(())
    }

    #[test]
    fn clean_existing_page_is_not_uploaded() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        atlas.mark_page_uploaded(0);
        let mut gpu = GpuGlyphAtlas { textures: vec![Some(texture(1))] };
        gpu.upload_dirty_pages_with(&mut atlas, |_, _, _, _| {
            panic!("clean page should not be uploaded")
        })?;
        assert_eq!(gpu.textures[0], Some(texture(1)));
        Ok(())
    }

    #[test]
    fn dirty_existing_page_is_updated_with_its_pixels() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        insert_full_page(&mut atlas, 7)?;
        let expected = atlas.page_data(0).unwrap().2.to_vec();
        let mut gpu = GpuGlyphAtlas { textures: vec![Some(texture(1))] };
        let mut calls = 0;
        gpu.upload_dirty_pages_with(&mut atlas, |existing, pixels, width, height| {
            calls += 1;
            assert_eq!(existing, Some(&texture(1)));
            assert_eq!((width, height), (8, 8));
            assert_eq!(pixels, expected);
            Ok(None)
        })?;
        assert_eq!(calls, 1);
        assert_eq!(gpu.textures[0], Some(texture(1)));
        assert!(!atlas.page_data(0).unwrap().3);
        Ok(())
    }

    #[test]
    fn multiple_pages_update_existing_and_create_missing_textures() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        insert_full_page(&mut atlas, 1)?;
        insert_full_page(&mut atlas, 2)?;
        assert_eq!(atlas.page_count(), 2);
        let mut gpu = GpuGlyphAtlas { textures: vec![Some(texture(1))] };
        let mut calls = Vec::new();
        gpu.upload_dirty_pages_with(&mut atlas, |existing, pixels, _, _| {
            calls.push(existing.is_some());
            assert!(pixels.contains(&(calls.len() as u8)));
            Ok(if existing.is_none() { Some(texture(2)) } else { None })
        })?;
        assert_eq!(calls, vec![true, false]);
        assert_eq!(gpu.textures, vec![Some(texture(1)), Some(texture(2))]);
        for index in 0..2 { assert!(!atlas.page_data(index).unwrap().3); }
        Ok(())
    }

    #[test]
    fn failed_upload_keeps_dirty_flag_and_existing_handle() -> Result<()> {
        for existing in [None, Some(texture(1))] {
            let mut atlas = GlyphAtlas::new(8, 8)?;
            let mut gpu = GpuGlyphAtlas { textures: vec![existing] };
            let result = gpu.upload_dirty_pages_with(&mut atlas, |_, _, _, _| {
                Err(anyhow!("simulated upload failure"))
            });
            assert!(result.is_err());
            assert!(atlas.page_data(0).unwrap().3);
            assert_eq!(gpu.textures[0], existing);
        }
        Ok(())
    }

    #[test]
    fn partial_failure_only_marks_successful_pages_uploaded() -> Result<()> {
        let mut atlas = GlyphAtlas::new(8, 8)?;
        for id in 1..=3 { insert_full_page(&mut atlas, id)?; }
        let mut gpu = GpuGlyphAtlas::new();
        let mut calls = 0;
        let result = gpu.upload_dirty_pages_with(&mut atlas, |_, _, _, _| {
            calls += 1;
            if calls == 2 { return Err(anyhow!("second page failed")); }
            Ok(Some(texture(1)))
        });
        assert!(result.is_err());
        assert_eq!(calls, 2);
        assert_eq!(gpu.textures, vec![Some(texture(1)), None, None]);
        assert!(!atlas.page_data(0).unwrap().3);
        assert!(atlas.page_data(1).unwrap().3);
        assert!(atlas.page_data(2).unwrap().3);
        Ok(())
    }
}
