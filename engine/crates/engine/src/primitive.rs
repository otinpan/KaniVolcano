use anyhow::{Result, anyhow, bail};
use cgmath::{InnerSpace, vec2, vec3};
use kani_volcano_math::Transform;
use renderer_vulkan::{SourceMesh, SourceTopology, SourceVertex, VertexLayout, VulkanRenderer};

use super::{EntityId, Material, MeshAssetId, MeshRenderer, Resources, Visibility, World};

pub type Vec3 = cgmath::Vector3<f32>;
pub type Vec2 = cgmath::Vector2<f32>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrimitiveType {
    Triangle,
    Rectangle,
    Cube,
    Circle,
    Polygon,
    Sphere,
    Line,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PrimitiveMesh {
    pub asset_id: MeshAssetId,
    pub primitive_type: PrimitiveType,
    pub vertex_layout: VertexLayout,
}

fn default_color() -> Vec3 {
    vec3(1.0, 1.0, 1.0)
}

#[derive(Clone, Debug, PartialEq)]
pub enum PrimitiveShape {
    Triangle {
        points: [Vec3; 3],
        color: Vec3,
    },
    Rectangle {
        points: [Vec3; 4],
        color: Vec3,
    },
    Cube {
        points: [Vec3; 8],
        color: Vec3,
    },
    Circle {
        radius: f32,
        segments: u32,
        color: Vec3,
    },
    Polygon {
        points: Vec<Vec3>,
        color: Vec3,
    },
    Sphere {
        radius: f32,
        rings: u32,    // the number of rings (parallel to the latitude)
        segments: u32, // the number of segments (parallel to the latitude)
        color: Vec3,
    },
    Line {
        pos0: Vec3,
        pos1: Vec3,
        color: Vec3,
    },
}

impl PrimitiveShape {
    pub fn primitive_type(&self) -> PrimitiveType {
        match self {
            Self::Triangle { .. } => PrimitiveType::Triangle,
            Self::Rectangle { .. } => PrimitiveType::Rectangle,
            Self::Cube { .. } => PrimitiveType::Cube,
            Self::Circle { .. } => PrimitiveType::Circle,
            Self::Polygon { .. } => PrimitiveType::Polygon,
            Self::Sphere { .. } => PrimitiveType::Sphere,
            Self::Line { .. } => PrimitiveType::Line,
        }
    }
}

// create mesh ////////////////////////////////////////////
pub unsafe fn create_primitive_mesh3d(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    let primitive_type = shape.primitive_type();
    let source = build_primitive_source(shape);
    let mesh_data = source.to_mesh3d_data();

    let handle = renderer.load_mesh_from_data(mesh_data, VertexLayout::Mesh3D)?;

    let asset_id = resources.insert_mesh_asset(handle, auto_release);
    Ok(PrimitiveMesh {
        asset_id,
        primitive_type,
        vertex_layout: VertexLayout::Mesh3D,
    })
}

pub unsafe fn create_primitive_debug_line(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    let primitive_type = shape.primitive_type();
    let source = build_primitive_source(shape);
    let mesh_data = source.to_debugline_data();

    let handle = renderer.load_mesh_from_data(mesh_data, VertexLayout::DebugLine3D)?;

    let asset_id = resources.insert_mesh_asset(handle, auto_release);

    Ok(PrimitiveMesh {
        asset_id,
        primitive_type,
        vertex_layout: VertexLayout::DebugLine3D,
    })
}

pub unsafe fn create_primitive_lit3d(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    let primitive_type = shape.primitive_type();
    let source = build_primitive_source(shape);
    let mesh_data = source.to_lit3d_data();

    let handle = renderer.load_mesh_from_data(mesh_data, VertexLayout::Lit3D)?;

    let asset_id = resources.insert_mesh_asset(handle, auto_release);

    Ok(PrimitiveMesh {
        asset_id,
        primitive_type,
        vertex_layout: VertexLayout::Lit3D,
    })
}

pub unsafe fn create_primitive_ui2d(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    let primitive_type = shape.primitive_type();
    let source = build_primitive_source(shape);
    let mesh_data = source.to_ui2d_data();

    let handle = renderer.load_mesh_from_data(mesh_data, VertexLayout::Ui2D)?;

    let asset_id = resources.insert_mesh_asset(handle, auto_release);

    Ok(PrimitiveMesh {
        asset_id,
        primitive_type,
        vertex_layout: VertexLayout::Ui2D,
    })
}

pub unsafe fn create_primitive_skybox(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    let primitive_type = shape.primitive_type();
    let source = build_primitive_source(shape);
    let mesh_data = source.to_skybox_data();

    let handle = renderer.load_mesh_from_data(mesh_data, VertexLayout::Skybox)?;
    let asset_id = resources.insert_mesh_asset(handle, auto_release);
    Ok(PrimitiveMesh {
        asset_id,
        primitive_type,
        vertex_layout: VertexLayout::Skybox,
    })
}

pub unsafe fn create_primitive_with_layout(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    vertex_layout: VertexLayout,
    auto_release: bool,
) -> Result<PrimitiveMesh> {
    match vertex_layout {
        VertexLayout::Mesh3D => create_primitive_mesh3d(renderer, resources, shape, auto_release),
        VertexLayout::DebugLine3D => {
            create_primitive_debug_line(renderer, resources, shape, auto_release)
        }
        VertexLayout::Lit3D => create_primitive_lit3d(renderer, resources, shape, auto_release),
        VertexLayout::Ui2D => create_primitive_ui2d(renderer, resources, shape, auto_release),
        VertexLayout::Skybox => create_primitive_skybox(renderer, resources, shape, auto_release),
    }
}

// build mesh. create vertices and indices from points //////////////////////////////////////////////////
pub fn build_primitive_source(shape: PrimitiveShape) -> SourceMesh {
    match shape {
        PrimitiveShape::Triangle { points, color } => build_triangle_source(points, color),
        PrimitiveShape::Rectangle { points, color } => build_rectangle_source(points, color),
        PrimitiveShape::Cube { points, color } => build_cube_source(points, color),
        PrimitiveShape::Circle {
            radius,
            segments,
            color,
        } => build_circle_source(radius, segments, color),
        PrimitiveShape::Polygon { points, color } => build_polygon_source(points, color),
        PrimitiveShape::Sphere {
            radius,
            rings,
            segments,
            color,
        } => build_sphere_source(radius, rings, segments, color),
        PrimitiveShape::Line { pos0, pos1, color } => build_line_source(pos0, pos1, color),
    }
}

fn build_triangle_source(points: [Vec3; 3], color: Vec3) -> SourceMesh {
    let normal = face_normal(points[0], points[1], points[2]);
    let vertices = vec![
        SourceVertex::new(points[0], color, vec2(0.0, 0.0), normal),
        SourceVertex::new(points[1], color, vec2(1.0, 0.0), normal),
        SourceVertex::new(points[2], color, vec2(0.5, 1.0), normal),
    ];

    let indices = vec![0, 1, 2];

    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}

fn build_rectangle_source(points: [Vec3; 4], color: Vec3) -> SourceMesh {
    let normal = face_normal(points[0], points[1], points[2]);
    let vertices = vec![
        SourceVertex::new(points[0], color, vec2(0.0, 0.0), normal),
        SourceVertex::new(points[1], color, vec2(1.0, 0.0), normal),
        SourceVertex::new(points[2], color, vec2(1.0, 1.0), normal),
        SourceVertex::new(points[3], color, vec2(0.0, 1.0), normal),
    ];

    let indices = vec![0, 1, 2, 2, 3, 0];
    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}

pub fn build_cube_source(points: [Vec3; 8], color: Vec3) -> SourceMesh {
    let faces = [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [2, 6, 7, 3],
        [1, 5, 6, 2],
        [0, 3, 7, 4],
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    for face in faces {
        let base = vertices.len() as u32;
        let normal = face_normal(points[face[0]], points[face[1]], points[face[2]]);

        vertices.push(SourceVertex::new(
            points[face[0]],
            color,
            vec2(0.0, 0.0),
            normal,
        ));
        vertices.push(SourceVertex::new(
            points[face[1]],
            color,
            vec2(1.0, 0.0),
            normal,
        ));
        vertices.push(SourceVertex::new(
            points[face[2]],
            color,
            vec2(1.0, 1.0),
            normal,
        ));
        vertices.push(SourceVertex::new(
            points[face[3]],
            color,
            vec2(0.0, 1.0),
            normal,
        ));

        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
    }
    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}

pub fn build_circle_source(radius: f32, segments: u32, color: Vec3) -> SourceMesh {
    let segments = segments.max(3);
    let normal = vec3(1.0, 0.0, 0.0);
    let mut vertices = Vec::with_capacity(segments as usize + 1);
    let mut indices = Vec::with_capacity(segments as usize * 3);

    vertices.push(SourceVertex::new(
        vec3(0.0, 0.0, 0.0),
        color,
        vec2(0.5, 0.5),
        normal,
    ));

    for i in 0..segments {
        let angle = std::f32::consts::TAU * i as f32 / segments as f32;
        let y = angle.cos() * radius;
        let z = angle.sin() * radius;
        let u = angle.cos() * 0.5 + 0.5;
        let v = angle.sin() * 0.5 + 0.5;

        vertices.push(SourceVertex::new(
            vec3(0.0, y, z),
            color,
            vec2(u, v),
            normal,
        ));
    }

    for i in 0..segments {
        let current = i + 1;
        let next = if i + 1 == segments { 1 } else { i + 2 };

        indices.extend_from_slice(&[0, current, next]);
    }
    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}

fn build_polygon_source(points: Vec<Vec3>, color: Vec3) -> SourceMesh {
    let size = points.len();

    if size < 3 {
        return SourceMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
            topology: SourceTopology::LineList,
        };
    }

    let mut vertices = Vec::with_capacity(size);
    let mut indices = Vec::with_capacity((size - 2) * 3);

    let mut min_y = points[0].y;
    let mut max_y = points[0].y;
    let mut min_z = points[0].z;
    let mut max_z = points[0].z;

    for p in &points {
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
        min_z = min_z.min(p.z);
        max_z = max_z.max(p.z);
    }

    let width = max_y - min_y;
    let height = max_z - min_z;
    let normal = polygon_normal_yz(&points);

    for p in &points {
        let u = if width.abs() > f32::EPSILON {
            (p.y - min_y) / width
        } else {
            0.0
        };
        let v = if height.abs() > f32::EPSILON {
            (p.z - min_z) / height
        } else {
            0.0
        };

        vertices.push(SourceVertex::new(*p, color, vec2(u, v), normal));
    }

    indices.extend(triangulate_polygon_yz(&points));
    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}

// create triangle from polygon using era clipping
fn triangulate_polygon_yz(points: &[Vec3]) -> Vec<u32> {
    let size = points.len();
    let mut indices = Vec::with_capacity((size - 2) * 3);
    let area = signed_area_yz(points);

    if area.abs() <= f32::EPSILON {
        return indices;
    }

    let mut remaining: Vec<usize> = if area > 0.0 {
        (0..size).collect()
    } else {
        (0..size).rev().collect()
    };

    while remaining.len() > 3 {
        let mut ear_index = None;

        for i in 0..remaining.len() {
            let prev = remaining[(i + remaining.len() - 1) % remaining.len()];
            let curr = remaining[i];
            let next = remaining[(i + 1) % remaining.len()];

            if is_ear_yz(points, prev, curr, next, &remaining) {
                ear_index = Some(i);
                indices.extend_from_slice(&[prev as u32, curr as u32, next as u32]);
                break;
            }
        }

        if let Some(i) = ear_index {
            remaining.remove(i);
        } else {
            return fan_triangulate(size);
        }
    }

    indices.extend_from_slice(&[
        remaining[0] as u32,
        remaining[1] as u32,
        remaining[2] as u32,
    ]);

    indices
}

fn signed_area_yz(points: &[Vec3]) -> f32 {
    let mut area = 0.0;

    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        area += current.y * next.z - next.y * current.z;
    }

    area * 0.5
}

fn is_ear_yz(points: &[Vec3], prev: usize, curr: usize, next: usize, remaining: &[usize]) -> bool {
    let a = points[prev];
    let b = points[curr];
    let c = points[next];

    if cross_yz(a, b, c) <= f32::EPSILON {
        return false;
    }

    for &index in remaining {
        if index == prev || index == curr || index == next {
            continue;
        }

        if point_in_triangle_yz(points[index], a, b, c) {
            return false;
        }
    }

    true
}

fn cross_yz(a: Vec3, b: Vec3, c: Vec3) -> f32 {
    (b.y - a.y) * (c.z - a.z) - (b.z - a.z) * (c.y - a.y)
}

fn point_in_triangle_yz(point: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let ab = cross_yz(a, b, point);
    let bc = cross_yz(b, c, point);
    let ca = cross_yz(c, a, point);

    ab >= -f32::EPSILON && bc >= -f32::EPSILON && ca >= -f32::EPSILON
}

fn fan_triangulate(size: usize) -> Vec<u32> {
    let mut indices = Vec::with_capacity((size - 2) * 3);

    for i in 1..(size - 1) {
        indices.extend_from_slice(&[0, i as u32, (i + 1) as u32]);
    }

    indices
}

fn build_sphere_source(radius: f32, rings: u32, segments: u32, color: Vec3) -> SourceMesh {
    let rings = rings.max(2);
    let segments = segments.max(3);

    let vertex_count = (rings + 1) * (segments + 1);
    let index_count = rings * segments * 6;

    let mut vertices = Vec::with_capacity(vertex_count as usize);
    let mut indices = Vec::with_capacity(index_count as usize);

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let theta = std::f32::consts::PI * v;

        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let phi = std::f32::consts::TAU * u;

            let x = radius * theta.sin() * phi.cos();
            let y = radius * theta.cos();
            let z = radius * theta.sin() * phi.sin();

            let position = vec3(x, y, z);
            let normal = normalized_or(position, vec3(0.0, 1.0, 0.0));

            vertices.push(SourceVertex::new(position, color, vec2(u, v), normal));
        }
    }

    let columns = segments + 1;

    for ring in 0..rings {
        for segment in 0..segments {
            let a = ring * columns + segment;
            let b = a + columns;
            let c = b + 1;
            let d = a + 1;

            // Match the outward vertex normals and counter-clockwise front faces.
            indices.extend_from_slice(&[a, d, b, d, c, b]);
        }
    }

    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::TriangleList,
    }
}


fn build_line_source(pos0: Vec3, pos1: Vec3, color: Vec3) -> SourceMesh {
    let normal = vec3(1.0, 0.0, 0.0);
    let vertices = vec![
        SourceVertex::new(pos0, color, vec2(0.0, 0.0), normal),
        SourceVertex::new(pos1, color, vec2(1.0, 1.0), normal),
    ];

    let indices = vec![0, 1];

    SourceMesh {
        vertices,
        indices,
        topology: SourceTopology::LineList,
    }
}

fn face_normal(p0: Vec3, p1: Vec3, p2: Vec3) -> Vec3 {
    normalized_or((p1 - p0).cross(p2 - p0), vec3(1.0, 0.0, 0.0))
}

fn polygon_normal_yz(points: &[Vec3]) -> Vec3 {
    if signed_area_yz(points) >= 0.0 {
        vec3(1.0, 0.0, 0.0)
    } else {
        vec3(-1.0, 0.0, 0.0)
    }
}

fn normalized_or(value: Vec3, fallback: Vec3) -> Vec3 {
    if value.magnitude2() <= f32::EPSILON {
        fallback
    } else {
        value.normalize()
    }
}

// spawn primitive object //////////////////////////////////////////////
pub fn spawn_primitive_from_mesh(
    world: &mut World,
    resources: &mut Resources,
    asset_id: MeshAssetId,
    material: Material,
    transform: Transform,
) -> Result<EntityId> {
    let mesh = resources
        .retain_mesh(asset_id)
        .ok_or_else(|| anyhow!("mesh asset not found: {asset_id:?}"))?;
    let mesh_renderer = MeshRenderer::new(mesh, material)?;
    let mesh_renderer = mesh_renderer.with_asset_id(asset_id);
    let entity = world.spawn();

    let added_transform = world.add_component(entity, transform);
    let added_mesh_renderer = world.add_component(entity, mesh_renderer);
    let added_visibility = world.add_component(entity, Visibility::default());
    let added_name = world.set_tags(entity, ["Primitive"]);

    if added_transform && added_mesh_renderer && added_visibility && added_name {
        Ok(entity)
    } else {
        bail!("failed to spawn primitive")
    }
}

pub unsafe fn spawn_shape_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    shape: PrimitiveShape,
    transform: Transform,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let vertex_layout = material.pipeline_key.required_vertex_layout();
    let primitive_mesh =
        create_primitive_with_layout(renderer, resources, shape, vertex_layout, auto_release)?;
    resources.register_primitive_mesh(primitive_mesh);

    spawn_primitive_from_mesh(
        world,
        resources,
        primitive_mesh.asset_id,
        material,
        transform,
    )
}

// create new object ////////////////////////////////////////
pub unsafe fn spawn_triangle_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let center = (p0 + p1 + p2) / 3.0;
    let shape = PrimitiveShape::Triangle {
        points: [p0 - center, p1 - center, p2 - center],
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: center,
        ..Default::default()
    };

    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

// parallel to yz
pub unsafe fn spawn_rectangle_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    pos: Vec3,
    width: f32,
    height: f32,
    rotation: Vec3,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    let shape = PrimitiveShape::Rectangle {
        points: [
            vec3(0.0, -half_width, half_height),
            vec3(0.0, -half_width, -half_height),
            vec3(0.0, half_width, -half_height),
            vec3(0.0, half_width, half_height),
        ],
        color: material.base_color.truncate(),
    };
    let transform = Transform {
        position: pos,
        rotation,
        ..Default::default()
    };
    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

pub unsafe fn spawn_cube_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    pos: Vec3,
    length: f32,
    rotation: Vec3,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let h = length * 0.5;
    let shape = PrimitiveShape::Cube {
        points: [
            vec3(h, -h, h),
            vec3(h, h, h),
            vec3(-h, h, h),
            vec3(-h, -h, h),
            vec3(h, -h, -h),
            vec3(h, h, -h),
            vec3(-h, h, -h),
            vec3(-h, -h, -h),
        ],
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: pos,
        rotation,
        ..Default::default()
    };

    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

// parallel to yz
pub unsafe fn spawn_circle_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    pos: Vec3,
    radius: f32,
    segments: u32,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let shape = PrimitiveShape::Circle {
        radius,
        segments,
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: pos,
        ..Default::default()
    };

    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

pub unsafe fn spawn_polygon_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    points: Vec<Vec3>,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    if points.is_empty() {
        bail!("Polygon must have at least one point.");
    }

    let center = points.iter().copied().sum::<Vec3>() / points.len() as f32;

    let local_points = points.iter().map(|p| *p - center).collect::<Vec<_>>();
    let shape = PrimitiveShape::Polygon {
        points: local_points,
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: center,
        ..Default::default()
    };

    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

pub unsafe fn spawn_sphere_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    center: Vec3,
    radius: f32,
    rings: u32,
    segments: u32,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let shape = PrimitiveShape::Sphere {
        radius,
        rings,
        segments,
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: center,
        ..Default::default()
    };
    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

pub unsafe fn spawn_line_with_material(
    world: &mut World,
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    pos0: Vec3,
    pos1: Vec3,
    material: Material,
    auto_release: bool,
) -> Result<EntityId> {
    let center = (pos0 + pos1) / 2.0;
    let shape = PrimitiveShape::Line {
        pos0: pos0 - center,
        pos1: pos1 - center,
        color: material.base_color.truncate(),
    };

    let transform = Transform {
        position: center,
        ..Default::default()
    };

    spawn_shape_with_material(
        world,
        renderer,
        resources,
        shape,
        transform,
        material,
        auto_release,
    )
}

// update mesh ///////////////////////////////////////////////////////////
// refered mesh in VulkanData update vertices and indices
pub unsafe fn update_primitive_mesh(
    renderer: &mut VulkanRenderer,
    resources: &mut Resources,
    mesh: PrimitiveMesh,
    shape: PrimitiveShape,
) -> Result<()> {
    if mesh.primitive_type != shape.primitive_type() {
        bail!(
            "Updated primitive_shape {:?} does not match mesh type {:?}.",
            shape.primitive_type(),
            mesh.primitive_type
        );
    }

    let source = build_primitive_source(shape);
    let mesh_handle = resources
        .get_mesh_handle(mesh.asset_id)
        .ok_or_else(|| anyhow!("mesh handle not found"))?;

    match mesh.vertex_layout {
        VertexLayout::Mesh3D => renderer.update_mesh_from_data(
            mesh_handle,
            source.to_mesh3d_data(),
            VertexLayout::Mesh3D,
        ),
        VertexLayout::DebugLine3D => renderer.update_mesh_from_data(
            mesh_handle,
            source.to_debugline_data(),
            VertexLayout::DebugLine3D,
        ),
        VertexLayout::Lit3D => {
            renderer.update_mesh_from_data(mesh_handle, source.to_lit3d_data(), VertexLayout::Lit3D)
        }
        VertexLayout::Ui2D => {
            renderer.update_mesh_from_data(mesh_handle, source.to_ui2d_data(), VertexLayout::Ui2D)
        }
        VertexLayout::Skybox => renderer.update_mesh_from_data(
            mesh_handle,
            source.to_skybox_data(),
            VertexLayout::Skybox,
        ),
    }
}
// test ///////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::vec3;
    use renderer_vulkan::MeshHandle;

    fn white() -> Vec3 {
        vec3(1.0, 1.0, 1.0)
    }

    #[test]
    fn sphere_triangle_winding_matches_outward_normals() {
        let sphere = build_sphere_source(1.0, 16, 24, vec3(1.0, 1.0, 1.0));
        let mut checked = 0;
        for triangle in sphere.indices.chunks_exact(3) {
            let a = sphere.vertices[triangle[0] as usize].pos;
            let b = sphere.vertices[triangle[1] as usize].pos;
            let c = sphere.vertices[triangle[2] as usize].pos;
            let face = (b - a).cross(c - a);
            // UV sphere poles contain degenerate triangles.
            if face.magnitude2() < 1e-10 {
                continue;
            }
            assert!(face.dot(a + b + c) > 0.0, "sphere triangle faces inward");
            checked += 1;
        }
        assert!(checked > 0);
    }

    #[test]
    fn check_face_normal() {
        assert_eq!(
            face_normal(
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, 1.0),
            ),
            vec3(1.0, 0.0, 0.0)
        );
        assert_eq!(
            face_normal(
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 1.0),
                vec3(0.0, 1.0, 0.0),
            ),
            vec3(-1.0, 0.0, 0.0)
        );
        assert_eq!(
            face_normal(
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            ),
            vec3(1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn primitive_shape_reports_matching_type() {
        let cases = [
            (
                PrimitiveShape::Triangle {
                    points: [
                        vec3(0.0, 0.0, 0.0),
                        vec3(0.0, 1.0, 0.0),
                        vec3(0.0, 0.0, 1.0),
                    ],
                    color: default_color(),
                },
                PrimitiveType::Triangle,
            ),
            (
                PrimitiveShape::Rectangle {
                    points: [
                        vec3(0.0, -1.0, 1.0),
                        vec3(0.0, -1.0, -1.0),
                        vec3(0.0, 1.0, -1.0),
                        vec3(0.0, 1.0, 1.0),
                    ],
                    color: default_color(),
                },
                PrimitiveType::Rectangle,
            ),
            (
                PrimitiveShape::Circle {
                    radius: 1.0,
                    segments: 8,
                    color: default_color(),
                },
                PrimitiveType::Circle,
            ),
            (
                PrimitiveShape::Sphere {
                    radius: 1.0,
                    rings: 32,
                    segments: 22,
                    color: default_color(),
                },
                PrimitiveType::Sphere,
            ),
        ];

        for (shape, primitive_type) in cases {
            assert_eq!(shape.primitive_type(), primitive_type);
        }
    }

    #[test]
    fn build_primitive_source_creates_expected_triangle_counts() {
        let source = build_primitive_source(PrimitiveShape::Triangle {
            points: [
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
                vec3(0.0, 0.0, 1.0),
            ],
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 3);
        assert_eq!(source.indices, vec![0, 1, 2]);
    }

    #[test]
    fn build_primitive_source_creates_expected_rectangle_counts() {
        let source = build_primitive_source(PrimitiveShape::Rectangle {
            points: [
                vec3(0.0, -1.0, 1.0),
                vec3(0.0, -1.0, -1.0),
                vec3(0.0, 1.0, -1.0),
                vec3(0.0, 1.0, 1.0),
            ],
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 4);
        assert_eq!(source.indices, vec![0, 1, 2, 2, 3, 0]);
    }

    #[test]
    fn build_primitive_source_creates_expected_cube_counts() {
        let source = build_primitive_source(PrimitiveShape::Cube {
            points: [
                vec3(0.5, -0.5, 0.5),
                vec3(0.5, 0.5, 0.5),
                vec3(-0.5, 0.5, 0.5),
                vec3(-0.5, -0.5, 0.5),
                vec3(0.5, -0.5, -0.5),
                vec3(0.5, 0.5, -0.5),
                vec3(-0.5, 0.5, -0.5),
                vec3(-0.5, -0.5, -0.5),
            ],
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 24);
        assert_eq!(source.indices.len(), 36);
    }

    #[test]
    fn build_primitive_source_creates_expected_circle_counts() {
        let source = build_primitive_source(PrimitiveShape::Circle {
            radius: 1.0,
            segments: 8,
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 9);
        assert_eq!(source.indices.len(), 24);
    }

    #[test]
    fn build_primitive_source_clamps_circle_segments_to_three() {
        let source = build_primitive_source(PrimitiveShape::Circle {
            radius: 1.0,
            segments: 1,
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 4);
        assert_eq!(source.indices.len(), 9);
    }

    #[test]
    fn build_primitive_source_creates_expected_polygon_counts() {
        let source = build_primitive_source(PrimitiveShape::Polygon {
            points: vec![
                vec3(0.0, -0.7, 0.7),
                vec3(0.0, -0.4, 0.5),
                vec3(0.0, 0.7, 0.5),
                vec3(0.0, 0.0, -0.6),
                vec3(0.0, -0.5, -0.4),
            ],
            color: default_color(),
        });

        assert_eq!(source.vertices.len(), 5);
        assert_eq!(source.indices.len(), 9);
    }

    #[test]
    fn build_primitive_source_returns_empty_polygon_for_too_few_points() {
        let source = build_primitive_source(PrimitiveShape::Polygon {
            points: vec![vec3(0.0, 0.0, 0.0), vec3(0.0, 1.0, 0.0)],
            color: default_color(),
        });

        assert!(source.vertices.is_empty());
        assert!(source.indices.is_empty());
    }

    #[test]
    fn spawn_primitive_from_mesh_accepts_matching_mesh3d_material() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let asset_id = resources.insert_mesh_asset(MeshHandle::new(0, VertexLayout::Mesh3D), false);

        let result = spawn_primitive_from_mesh(
            &mut world,
            &mut resources,
            asset_id,
            Material {
                pipeline_key: renderer_vulkan::PipelineKey::Mesh3D,
                ..Material::default()
            },
            Transform::default(),
        );

        let entity = result.unwrap();
        assert!(world.has_component::<Visibility>(entity));
    }

    #[test]
    fn spawn_primitive_from_mesh_accepts_matching_debugline_material() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let asset_id =
            resources.insert_mesh_asset(MeshHandle::new(0, VertexLayout::DebugLine3D), false);

        let result = spawn_primitive_from_mesh(
            &mut world,
            &mut resources,
            asset_id,
            Material {
                pipeline_key: renderer_vulkan::PipelineKey::DebugLine3D,
                ..Material::default()
            },
            Transform::default(),
        );

        let entity = result.unwrap();
        assert!(world.has_component::<Visibility>(entity));
    }

    #[test]
    fn spawn_primitive_from_mesh_rejects_mismatched_pipeline_and_vertex_layout() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let asset_id = resources.insert_mesh_asset(MeshHandle::new(0, VertexLayout::Mesh3D), false);

        let result = spawn_primitive_from_mesh(
            &mut world,
            &mut resources,
            asset_id,
            Material {
                pipeline_key: renderer_vulkan::PipelineKey::DebugLine3D,
                ..Material::default()
            },
            Transform::default(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn check_face_nomal() {}
}
