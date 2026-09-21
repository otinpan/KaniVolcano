use std::collections::{HashMap, HashSet};
use anyhow::Result;
use renderer_vulkan::{GpuTextMesh, TextRenderItem};
use kani_volcano_math::Transform;
use crate::component::{Text, Visibility};

use crate::{EntityId, FontAssetId};
use super::RenderContext;

#[derive(Clone, PartialEq)]
pub struct TextLayout{
    pub content: String,
    pub font: FontAssetId,
    pub font_size: f32,
    pub line_height: f32,
}

struct CachedText{
    layout: TextLayout,
    mesh: GpuTextMesh,
}

#[derive(Default)]
pub struct TextRenderSystem{
    cache: HashMap<EntityId, CachedText>,
}

struct TextSnapshot {
    entity: EntityId,
    layout: TextLayout,
    transform: Transform,
    color: [f32; 4],
    alpha: f32,
}

fn collect_text_entities(context: &RenderContext<'_>) -> HashSet<EntityId> {
    context.query1::<Text>().map(|(entity, _)| entity).collect()
}

fn collect_renderable_texts(context: &RenderContext<'_>) -> Vec<TextSnapshot> {
    context.query2::<Text, Transform>()
        .filter(|(entity, _, _)| {
            context.get_component::<Visibility>(*entity)
                .is_none_or(|visibility| visibility.is_visible)
        })
        .map(|(entity, text, transform)| TextSnapshot {
            entity,
            layout: TextLayout {
                content: text.content.clone(),
                font: text.font,
                font_size: text.font_size,
                line_height: text.line_height,
            },
            transform: transform.clone(),
            color: [text.color.x, text.color.y, text.color.z, 1.0],
            alpha: text.alpha,
        })
        .collect()
}

impl TextRenderSystem{
    pub fn update(&mut self, context: &mut RenderContext<'_>) -> Result<()>{
        // Own snapshots so no World borrow remains during GPU updates.
        let text_entities = collect_text_entities(context);
        let texts = collect_renderable_texts(context);
        context.set_text_render_items(Vec::new());
        unsafe { self.release_removed(context, &text_entities)?; }

        let mut items = Vec::new();
        for text in texts {
            if self.need_rebuild(text.entity, &text.layout) {
                unsafe {
                    let mut mesh = context.create_text_mesh(
                        text.layout.font, &text.layout.content,
                        text.layout.font_size, text.layout.line_height,
                    )?;
                    if let Err(error) = self.store_mesh(context, text.entity, text.layout, &mut mesh) {
                        context.destroy_text_mesh(&mut mesh)?;
                        return Err(error);
                    }
                }
            }
            if let Some(cached) = self.cache.get(&text.entity) {
                for batch in &cached.mesh.batches {
                    items.push(TextRenderItem {
                        mesh: batch.mesh,
                        atlas_page: batch.page,
                        transform: text.transform.clone(),
                        color: text.color,
                        alpha: text.alpha,
                    });
                }
            }
        }
        context.set_text_render_items(items);
        Ok(())
    }
    // if layout data is updated, mesh need to rebuild
    pub fn need_rebuild(
        &self,
        entity: EntityId,
        layout: &TextLayout
    ) -> bool{
        self.cache
            .get(&entity)
            .is_none_or(|cached| cached.layout!=*layout)
    }

    pub unsafe fn store_mesh(
        &mut self,
        context: &mut RenderContext<'_>,
        entity: EntityId,
        layout: TextLayout,
        new_mesh: &mut GpuTextMesh,
    ) -> Result<()>{
        if let Some(cached)=self.cache.get_mut(&entity){
            context.destroy_text_mesh(&mut cached.mesh)?;
        }

        self.cache.insert(entity,CachedText { 
            layout, 
            mesh: std::mem::take(new_mesh) 
        });

        Ok(())
    }

    pub unsafe fn release_removed(
        &mut self,
        context: &mut RenderContext<'_>,
        text_entities: &HashSet<EntityId>,
    ) -> Result<()>{
        let removed: Vec<EntityId>=self.cache.keys()
            .copied()
            .filter(|entity| !text_entities.contains(entity))
            .collect();

        for entity in removed{
            if let Some(cached) = self.cache.get_mut(&entity){
                context.destroy_text_mesh(&mut cached.mesh)?;
            }

            self.cache.remove(&entity);
        }
        Ok(())
    }

    pub unsafe fn clear(
        &mut self,
        context: &mut RenderContext<'_>,
    ) -> Result<()>{
        self.release_removed(context, &HashSet::new())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_layout_rebuilds_only_when_layout_changes() {
        let entity = EntityId(0);
        let layout = TextLayout {
            content: "Hello".into(),
            font: FontAssetId(0),
            font_size: 24.0,
            line_height: 30.0,
        };
        let mut system = TextRenderSystem::default();
        assert!(system.need_rebuild(entity, &layout));
        system.cache.insert(entity, CachedText {
            layout: layout.clone(),
            mesh: GpuTextMesh::default(),
        });
        assert!(!system.need_rebuild(entity, &layout));
        assert!(system.need_rebuild(EntityId(1), &layout));

        let mut changed = layout.clone();
        changed.content.push('!');
        assert!(system.need_rebuild(entity, &changed));
        changed = layout.clone();
        changed.font = FontAssetId(1);
        assert!(system.need_rebuild(entity, &changed));
        changed = layout.clone();
        changed.font_size = 32.0;
        assert!(system.need_rebuild(entity, &changed));
        changed = layout.clone();
        changed.line_height = 40.0;
        assert!(system.need_rebuild(entity, &changed));
    }
}