//! Small, host-visible text meshes. Never overwrite a buffer referenced by an
//! outstanding submission. Frame bits are cleared only after its fence wait.
use crate::{
    MeshHandle,
    types::{Mesh, VulkanData},
    vertex::{Ui2DVertex, VertexLayout},
};
use anyhow::Result;
use std::collections::HashMap;
use vulkanalia::prelude::v1_0::*;

const CACHE_BYTES: u64 = 16 * 1024 * 1024;
const CACHE_MESHES: usize = 1024;
const _: () = assert!(crate::MAX_FRAMES_IN_FLIGHT <= 64);

#[derive(Default)]
pub(crate) struct TextBuffers {
    live: HashMap<usize, u64>,
    retired: Vec<(usize, u64)>,
    ready: Vec<usize>,
    free_slots: Vec<usize>,
}

impl TextBuffers {
    pub fn owns(&self, index: usize) -> bool {
        self.live.contains_key(&index)
            || self.ready.contains(&index)
            || self.retired.iter().any(|(slot, _)| *slot == index)
            || self.free_slots.contains(&index)
    }

    pub fn submitted(&mut self, index: usize, frame: usize) {
        if let Some(mask) = self.live.get_mut(&index) {
            *mask |= 1 << frame;
        }
    }

    pub fn retire(&mut self, index: usize) {
        if let Some(mask) = self.live.remove(&index) {
            if mask == 0 {
                self.ready.push(index);
            } else {
                self.retired.push((index, mask));
            }
        }
    }

    pub fn completed(&mut self, frame: usize) {
        let bit = !(1 << frame);
        for mask in self.live.values_mut() {
            *mask &= bit;
        }
        self.retired.retain_mut(|(index, mask)| {
            *mask &= bit;
            if *mask == 0 {
                self.ready.push(*index);
                false
            } else {
                true
            }
        });
    }

    // Only ready (never in-flight) allocations count toward the spare cache.
    pub unsafe fn trim(&mut self, device: &Device, data: &mut VulkanData) {
        let mut bytes = 0;
        let mut count = 0;
        self.ready.retain(|index| {
            let mesh = data.meshes[*index].as_ref().unwrap();
            let size = mesh.vertex_buffer_size + mesh.index_buffer_size;
            if bytes + size <= CACHE_BYTES && count < CACHE_MESHES {
                bytes += size;
                count += 1;
                true
            } else {
                destroy(device, data.meshes[*index].take().unwrap());
                self.free_slots.push(*index);
                false
            }
        });
    }

    pub unsafe fn upload(
        &mut self,
        instance: &Instance,
        device: &Device,
        data: &mut VulkanData,
        vertices: &[Ui2DVertex],
        indices: &[u32],
    ) -> Result<MeshHandle> {
        anyhow::ensure!(
            !vertices.is_empty() && !indices.is_empty(),
            "empty text batch"
        );
        let index_count = u32::try_from(indices.len())?;
        let vertex_bytes = std::mem::size_of_val(vertices) as u64;
        let index_bytes = std::mem::size_of_val(indices) as u64;
        let candidate = self
            .ready
            .iter()
            .enumerate()
            .filter_map(|(position, index)| {
                let mesh = data.meshes[*index].as_ref()?;
                (mesh.vertex_buffer_size >= vertex_bytes && mesh.index_buffer_size >= index_bytes)
                    .then_some((position, mesh.vertex_buffer_size + mesh.index_buffer_size))
            })
            .min_by_key(|(_, size)| *size);
        let index = if let Some((position, _)) = candidate {
            self.ready.swap_remove(position)
        } else {
            let vertex_capacity = vertex_bytes
                .checked_next_power_of_two()
                .unwrap_or(vertex_bytes);
            let index_capacity = index_bytes
                .checked_next_power_of_two()
                .unwrap_or(index_bytes);
            let (vertex_buffer, vertex_buffer_memory) = allocate(
                instance,
                device,
                data,
                vertex_capacity,
                vk::BufferUsageFlags::VERTEX_BUFFER,
            )?;
            let (index_buffer, index_buffer_memory) = match allocate(
                instance,
                device,
                data,
                index_capacity,
                vk::BufferUsageFlags::INDEX_BUFFER,
            ) {
                Ok(pair) => pair,
                Err(error) => {
                    device.destroy_buffer(vertex_buffer, None);
                    device.free_memory(vertex_buffer_memory, None);
                    return Err(error);
                }
            };
            let mesh = Mesh {
                vertex_buffer,
                vertex_buffer_memory,
                vertex_buffer_size: vertex_capacity,
                index_buffer,
                index_buffer_memory,
                index_buffer_size: index_capacity,
                index_count,
                vertex_layout: VertexLayout::Ui2D,
            };
            if let Some(index) = self.free_slots.pop() {
                data.meshes[index] = Some(mesh);
                index
            } else {
                let index = data.meshes.len();
                data.meshes.push(Some(mesh));
                index
            }
        };
        let mesh = data.meshes[index].as_mut().unwrap();
        let result = write(device, mesh.vertex_buffer_memory, vertices)
            .and_then(|_| write(device, mesh.index_buffer_memory, indices));
        if let Err(error) = result {
            self.ready.push(index);
            self.trim(device, data);
            return Err(error);
        }
        mesh.index_count = index_count;
        self.live.insert(index, 0);
        Ok(MeshHandle::new(index, VertexLayout::Ui2D))
    }
}

unsafe fn allocate(
    instance: &Instance,
    device: &Device,
    data: &VulkanData,
    size: u64,
    usage: vk::BufferUsageFlags,
) -> Result<(vk::Buffer, vk::DeviceMemory)> {
    let info = vk::BufferCreateInfo::builder()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = device.create_buffer(&info, None)?;
    let requirements = device.get_buffer_memory_requirements(buffer);
    let properties = instance.get_physical_device_memory_properties(data.physical_device);
    let flags = vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
    let memory_type = (0..properties.memory_type_count).find(|&index| {
        requirements.memory_type_bits & (1 << index) != 0
            && properties.memory_types[index as usize]
                .property_flags
                .contains(flags)
    });
    let Some(memory_type) = memory_type else {
        device.destroy_buffer(buffer, None);
        anyhow::bail!("no coherent host-visible memory for text buffers");
    };
    let info = vk::MemoryAllocateInfo::builder()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type);
    let memory = match device.allocate_memory(&info, None) {
        Ok(memory) => memory,
        Err(error) => {
            device.destroy_buffer(buffer, None);
            return Err(error.into());
        }
    };
    if let Err(error) = device.bind_buffer_memory(buffer, memory, 0) {
        device.destroy_buffer(buffer, None);
        device.free_memory(memory, None);
        return Err(error.into());
    }
    Ok((buffer, memory))
}

unsafe fn write<T: Copy>(device: &Device, memory: vk::DeviceMemory, values: &[T]) -> Result<()> {
    let target = device.map_memory(
        memory,
        0,
        std::mem::size_of_val(values) as u64,
        vk::MemoryMapFlags::empty(),
    )?;
    std::ptr::copy_nonoverlapping(values.as_ptr(), target.cast(), values.len());
    device.unmap_memory(memory);
    // HOST_COHERENT writes become available to vertex/index reads on submission.
    Ok(())
}

unsafe fn destroy(device: &Device, mesh: Mesh) {
    device.destroy_buffer(mesh.index_buffer, None);
    device.free_memory(mesh.index_buffer_memory, None);
    device.destroy_buffer(mesh.vertex_buffer, None);
    device.free_memory(mesh.vertex_buffer_memory, None);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retired_mesh_waits_for_every_frame_that_used_it() {
        let mut pool = TextBuffers::default();
        pool.live.insert(3, 0);
        pool.submitted(3, 0);
        pool.submitted(3, 1);
        pool.retire(3);
        pool.completed(0);
        assert!(pool.ready.is_empty());
        pool.completed(0); // reusing a fence must not complete another frame
        assert!(pool.ready.is_empty());
        pool.completed(1);
        assert_eq!(pool.ready, vec![3]);
        assert!(pool.retired.is_empty());
    }
    #[test]
    fn updates_without_rendering_can_be_reused_immediately() {
        let mut pool = TextBuffers::default();
        pool.live.insert(7, 0);
        pool.retire(7);
        assert_eq!(pool.ready, vec![7]);
        pool.retire(7); // destruction remains idempotent until slot reuse
        assert!(pool.owns(7));
        assert_eq!(pool.ready, vec![7]);
    }
    #[test]
    fn completed_live_mesh_tracks_subsequent_submission() {
        let mut pool = TextBuffers::default();
        pool.live.insert(2, 0);
        pool.submitted(2, 0);
        pool.completed(0);
        pool.submitted(2, 1);
        pool.retire(2);
        pool.completed(0);
        assert!(pool.ready.is_empty());
        pool.completed(1);
        assert_eq!(pool.ready, vec![2]);
    }
}
