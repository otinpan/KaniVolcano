use crate::types::*;
use anyhow::{bail, ensure, Context, Result};


pub fn load_gltf(path: &str) -> Result<GltfModel> {
    let (document, buffers, images) = gltf::import(path)?;

    // Scene
    let mut scenes = Vec::new();
    for scene in document.scenes() {
        scenes.push(parse_scene(scene)?);
    }

    let default_scene = document.default_scene().map(|scene| scene.index());

    // Node
    let mut nodes = Vec::new();
    for node in document.nodes() {
        nodes.push(parse_node(node)?);
    }

    // Mesh
    let mut meshes = Vec::new();
    for mesh in document.meshes() {
        meshes.push(parse_mesh(mesh, &buffers)?);
    }

    // Material
    let mut materials = Vec::new();
    for material in document.materials() {
        materials.push(parse_material(material))
    }

    let mut textures = Vec::new();
    for texture in document.textures() {
        textures.push(parse_texture(texture));
    }
    let mut skins = Vec::new();
    for skin in document.skins() {
        skins.push(parse_skin(skin, &buffers)?);
    }
    let mut animations = Vec::new();
    for animation in document.animations() {
        animations.push(parse_animation(animation, &buffers)?);
    }
    Ok(GltfModel {
        scenes,
        default_scene,
        nodes,
        meshes,
        materials,
        textures,
        images,
        skins,
        animations,
    })
}

fn parse_scene(scene: gltf::Scene<'_>) -> Result<ModelScene> {
    Ok(ModelScene {
        name: scene.name().map(str::to_owned),
        roots: scene.nodes().map(|node| node.index()).collect(),
    })
}

fn parse_node(node: gltf::Node<'_>) -> Result<ModelNode> {
    Ok(ModelNode {
        name: node.name().map(str::to_owned),
        children: node.children().map(|child| child.index()).collect(),
        mesh: node.mesh().map(|mesh| mesh.index()),
        skin: node.skin().map(|skin| skin.index()),
        transform: parse_transform(node.transform()),
    })
}

fn parse_transform(transform: gltf::scene::Transform) -> NodeTransform {
    match transform {
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => NodeTransform::Trs {
            translation,
            rotation,
            scale,
        },

        gltf::scene::Transform::Matrix { matrix } => NodeTransform::Matrix(matrix),
    }
}

fn parse_mesh(mesh: gltf::Mesh<'_>, buffers: &[gltf::buffer::Data]) -> Result<ModelMesh> {
    let mut primitives = Vec::new();
    for primitive in mesh.primitives() {
        primitives.push(parse_primitive(primitive, buffers)?);
    }
    Ok(ModelMesh {
        name: mesh.name().map(str::to_owned),
        primitives,
    })
}

fn parse_primitive(
    primitive: gltf::Primitive<'_>,
    buffers: &[gltf::buffer::Data],
) -> Result<ModelPrimitive> {
    ensure!(
        primitive.morph_targets().next().is_none(),
        "Morph targets are not supported"
    );
    let topology = match primitive.mode() {
        gltf::mesh::Mode::Points => PrimitiveTopology::Points,
        gltf::mesh::Mode::Lines => PrimitiveTopology::Lines,
        gltf::mesh::Mode::Triangles => PrimitiveTopology::Triangles,
        mode => bail!("Unsupported primitive mode: {mode:?}"),
    };
    let reader =
        primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
    let joints = reader
        .read_joints(0)
        .map(|values| values.into_u16().collect());
    let weights = reader
        .read_weights(0)
        .map(|values| values.into_f32().collect());
    ensure!(
        joints.is_some() == weights.is_some(),
        "JOINTS_0 and WEIGHTS_0 must be present together"
    );
    Ok(ModelPrimitive {
        positions: reader
            .read_positions()
            .context("Missing or unreadable POSITION")?
            .collect(),
        normal: reader.read_normals().map(|values| values.collect()),
        joints,
        weights,
        tex_coord: reader
            .read_tex_coords(0)
            .map(|values| values.into_f32().collect()),
        indices: reader
            .read_indices()
            .map(|values| values.into_u32().collect()),
        material: primitive.material().index(),
        topology,
    })
}

fn parse_texture_ref(info: gltf::texture::Info<'_>) -> TextureRef {
    TextureRef {
        texture: info.texture().index(),
        tex_coord: info.tex_coord(),
    }
}

fn parse_material(material: gltf::Material<'_>) -> ModelMaterial {
    let pbr = material.pbr_metallic_roughness();
    let normal = material.normal_texture();
    let occlusion = material.occlusion_texture();
    ModelMaterial {
        name: material.name().map(str::to_owned),
        base_color: pbr.base_color_factor(),
        base_color_texture: pbr.base_color_texture().map(parse_texture_ref),
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        metallic_roughness_texture: pbr.metallic_roughness_texture().map(parse_texture_ref),
        normal_scale: normal.as_ref().map_or(1.0, |info| info.scale()),
        normal_texture: normal.map(|info| TextureRef {
            texture: info.texture().index(),
            tex_coord: info.tex_coord(),
        }),
        occlusion_strength: occlusion.as_ref().map_or(1.0, |info| info.strength()),
        occlusion_texture: occlusion.map(|info| TextureRef {
            texture: info.texture().index(),
            tex_coord: info.tex_coord(),
        }),
        emissive: material.emissive_factor(),
        emissive_texture: material.emissive_texture().map(parse_texture_ref),
        alpha_mode: match material.alpha_mode() {
            gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
            gltf::material::AlphaMode::Mask => AlphaMode::Mask,
            gltf::material::AlphaMode::Blend => AlphaMode::Blend,
        },
        alpha_cutoff: material.alpha_cutoff().unwrap_or(0.5),
        double_sided: material.double_sided(),
    }
}

fn parse_texture(texture: gltf::Texture<'_>) -> ModelTexture {
    let sampler = texture.sampler();
    ModelTexture {
        image: texture.source().index(),
        sampler: ModelSampler {
            mag_filter: sampler.mag_filter(),
            min_filter: sampler.min_filter(),
            wrap_s: sampler.wrap_s(),
            wrap_t: sampler.wrap_t(),
        },
    }
}

fn parse_skin(skin: gltf::Skin<'_>, buffers: &[gltf::buffer::Data]) -> Result<ModelSkin> {
    let joints: Vec<_> = skin.joints().map(|node| node.index()).collect();
    let reader = skin.reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
    let inverse_bind_matrices: Vec<_> = if skin.inverse_bind_matrices().is_some() {
        reader
            .read_inverse_bind_matrices()
            .context("Unreadable inverse bind matrices")?
            .collect()
    } else {
        let identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        vec![identity; joints.len()]
    };
    ensure!(
        inverse_bind_matrices.len() == joints.len(),
        "Inverse bind matrix count must match joint count"
    );
    Ok(ModelSkin {
        name: skin.name().map(str::to_owned),
        joints,
        inverse_bind_matrices,
        skeleton: skin.skeleton().map(|node| node.index()),
    })
}

fn parse_animation(
    animation: gltf::Animation<'_>,
    buffers: &[gltf::buffer::Data],
) -> Result<ModelAnimation> {
    let mut tracks = Vec::new();
    let mut duration = 0.0_f32;
    for channel in animation.channels() {
        let track = parse_animation_track(channel, buffers)?;
        if let Some(&end) = track.time.last() {
            duration = duration.max(end);
        }
        tracks.push(track);
    }
    Ok(ModelAnimation {
        name: animation.name().map(str::to_owned),
        duration,
        tracks,
    })
}

fn parse_animation_track(
    channel: gltf::animation::Channel<'_>,
    buffers: &[gltf::buffer::Data],
) -> Result<AnimationTrack> {
    use gltf::animation::util::ReadOutputs;
    ensure!(
        channel.target().property() != gltf::animation::Property::MorphTargetWeights,
        "Morph weight animation is not supported"
    );
    let reader = channel.reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));
    let time: Vec<f32> = reader
        .read_inputs()
        .context("Unreadable animation times")?
        .collect();
    ensure!(
        !time.is_empty()
            && time.iter().all(|t| t.is_finite() && *t >= 0.0)
            && time.windows(2).all(|pair| pair[0] < pair[1]),
        "Animation times must be nonempty, finite, nonnegative, and strictly increasing"
    );
    let interpolation = match channel.sampler().interpolation() {
        gltf::animation::Interpolation::Step => Interpolation::Step,
        gltf::animation::Interpolation::Linear => Interpolation::Linear,
        gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
    };
    // Preserve cubic spline tangents, including unnormalized rotation tangents.
    let values = match reader
        .read_outputs()
        .context("Unreadable animation values")?
    {
        ReadOutputs::Translations(values) => AnimationValues::Translation(values.collect()),
        ReadOutputs::Rotations(values) => AnimationValues::Rotation(values.into_f32().collect()),
        ReadOutputs::Scales(values) => AnimationValues::Scale(values.collect()),
        ReadOutputs::MorphTargetWeights(_) => bail!("Morph weight animation is not supported"),
    };
    let count = match &values {
        AnimationValues::Translation(values) | AnimationValues::Scale(values) => values.len(),
        AnimationValues::Rotation(values) => values.len(),
    };
    let multiplier = if matches!(interpolation, Interpolation::CubicSpline) {
        3
    } else {
        1
    };
    ensure!(
        count == time.len() * multiplier,
        "Animation value count does not match key times"
    );
    Ok(AnimationTrack {
        node: channel.target().node().index(),
        time,
        interpolation,
        values,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_both_containers_and_preserves_scene_and_skin() {
        let json = br#"{"asset":{"version":"2.0"},"scene":1,"scenes":[{"nodes":[]},{"nodes":[0]}],"nodes":[{"children":[1]},{"translation":[1,2,3]}],"skins":[{"joints":[1]}]}"#;
        let base = std::env::temp_dir().join(format!("kani-loader-{}", std::process::id()));
        let mut padded = json.to_vec();
        while padded.len() % 4 != 0 {
            padded.push(b' ');
        }
        let mut glb = Vec::new();
        for word in [
            0x46546c67_u32,
            2,
            (20 + padded.len()) as u32,
            padded.len() as u32,
            0x4e4f534a,
        ] {
            glb.extend(word.to_le_bytes());
        }
        glb.extend(padded);
        for (extension, bytes) in [("gltf", json.as_slice()), ("glb", glb.as_slice())] {
            let path = base.with_extension(extension);
            std::fs::write(&path, bytes).unwrap();
            let result = load_gltf(path.to_str().unwrap());
            std::fs::remove_file(&path).unwrap();
            let model = result.unwrap();
            assert_eq!(model.default_scene, Some(1));
            assert_eq!(model.scenes[1].roots, vec![0]);
            assert_eq!(model.nodes[0].children, vec![1]);
            assert_eq!(model.skins[0].joints, vec![1]);
            assert_eq!(
                model.skins[0].inverse_bind_matrices[0][3],
                [0.0, 0.0, 0.0, 1.0]
            );
            assert!(matches!(
                model.nodes[1].transform,
                NodeTransform::Trs {
                    translation: [1.0, 2.0, 3.0],
                    ..
                }
            ));
        }
    }

    #[test]
    fn reads_mesh_and_animation_buffers() {
        let json = br#"{
            "asset":{"version":"2.0"},"nodes":[{}],
            "buffers":[{"byteLength":68}],
            "bufferViews":[
                {"buffer":0,"byteOffset":0,"byteLength":36},
                {"buffer":0,"byteOffset":36,"byteLength":8},
                {"buffer":0,"byteOffset":44,"byteLength":24}
            ],
            "accessors":[
                {"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},
                {"bufferView":1,"componentType":5126,"count":2,"type":"SCALAR","min":[0],"max":[2]},
                {"bufferView":2,"componentType":5126,"count":2,"type":"VEC3"}
            ],
            "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
            "animations":[{
                "samplers":[{"input":1,"output":2}],
                "channels":[{"sampler":0,"target":{"node":0,"path":"translation"}}]
            }]
        }"#;
        let document = gltf::Gltf::from_slice(json).unwrap().document;
        let values: [f32; 17] = [
            0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0,
        ];
        let buffers = [gltf::buffer::Data(
            values.iter().flat_map(|v| v.to_le_bytes()).collect(),
        )];
        let mesh = parse_mesh(document.meshes().next().unwrap(), &buffers).unwrap();
        assert_eq!(mesh.primitives[0].positions[1], [1.0, 0.0, 0.0]);
        assert!(mesh.primitives[0].indices.is_none());
        let animation = parse_animation(document.animations().next().unwrap(), &buffers).unwrap();
        assert_eq!(animation.duration, 2.0);
        assert_eq!(animation.tracks[0].time, vec![0.0, 2.0]);
        match &animation.tracks[0].values {
            AnimationValues::Translation(values) => assert_eq!(values[1], [3.0, 0.0, 0.0]),
            _ => panic!("Expected translation track"),
        }
    }
}