//! glTF importer for Chronos Engine Phase 10.
//!
//! Loads `.glb` / `.gltf` files into engine-compatible data structures.
//! Supports PBR materials, skeletal animation, scene hierarchies,
//! and skin data. Produces standalone types compatible with the engine's
//! `render3d`, `material`, and `skeletal` modules.

#[cfg(feature = "asset-pipeline")]
use std::fmt;
#[cfg(feature = "asset-pipeline")]
#[allow(unused_imports)]
use std::path::{Path, PathBuf};

#[cfg(feature = "asset-pipeline")]
use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// Error Type
// ──────────────────────────────────────────────

/// Errors that can occur during glTF import.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug)]
pub enum GltfImportError {
    /// Filesystem or I/O failure.
    Io(std::io::Error),
    /// The file could not be parsed as valid glTF.
    Parse(String),
    /// A mesh primitive was missing required data or was malformed.
    InvalidMesh(String),
    /// A material reference could not be resolved.
    InvalidMaterial(String),
    /// A required buffer was missing or could not be loaded.
    MissingBuffer(String),
    /// An optional glTF feature is not supported by this importer.
    UnsupportedFeature(String),
}

#[cfg(feature = "asset-pipeline")]
impl fmt::Display for GltfImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GltfImportError::Io(e) => write!(f, "glTF I/O error: {}", e),
            GltfImportError::Parse(msg) => write!(f, "glTF parse error: {}", msg),
            GltfImportError::InvalidMesh(msg) => write!(f, "glTF invalid mesh: {}", msg),
            GltfImportError::InvalidMaterial(msg) => write!(f, "glTF invalid material: {}", msg),
            GltfImportError::MissingBuffer(msg) => write!(f, "glTF missing buffer: {}", msg),
            GltfImportError::UnsupportedFeature(msg) => {
                write!(f, "glTF unsupported feature: {}", msg)
            }
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl std::error::Error for GltfImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GltfImportError::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(feature = "asset-pipeline")]
impl From<std::io::Error> for GltfImportError {
    fn from(e: std::io::Error) -> Self {
        GltfImportError::Io(e)
    }
}

#[cfg(feature = "asset-pipeline")]
impl From<gltf::Error> for GltfImportError {
    fn from(e: gltf::Error) -> Self {
        GltfImportError::Parse(e.to_string())
    }
}

// ──────────────────────────────────────────────
// Vertex & Morph Target
// ──────────────────────────────────────────────

/// A single vertex with position, normal, and UV coordinates.
/// Layout-compatible with `render3d::Vertex3D` (x, y, z, nx, ny, nz, u, v).
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GltfVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub nx: f32,
    pub ny: f32,
    pub nz: f32,
    pub u: f32,
    pub v: f32,
}

#[cfg(feature = "asset-pipeline")]
impl GltfVertex {
    fn new(pos: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> Self {
        GltfVertex {
            x: pos[0],
            y: pos[1],
            z: pos[2],
            nx: normal[0],
            ny: normal[1],
            nz: normal[2],
            u: uv[0],
            v: uv[1],
        }
    }
}

/// Displacements for a morph target (blend shape).
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfMorphTarget {
    /// Per-vertex position deltas.
    pub position_deltas: Vec<[f32; 3]>,
    /// Per-vertex normal deltas (may be empty).
    pub normal_deltas: Vec<[f32; 3]>,
    /// Per-vertex tangent deltas (may be empty).
    pub tangent_deltas: Vec<[f32; 3]>,
}

// ──────────────────────────────────────────────
// Mesh
// ──────────────────────────────────────────────

/// A mesh extracted from glTF, containing one or more primitives flattened
/// into a single vertex / index buffer pair.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfMesh {
    /// Name from the glTF asset (may be auto-generated).
    pub name: String,
    /// Interleaved vertices: position, normal, UV.
    pub vertices: Vec<GltfVertex>,
    /// Triangle-list indices.
    pub indices: Vec<u32>,
    /// Index into `GltfScene::materials`, if a material was assigned.
    pub material_index: Option<usize>,
    /// Optional morph targets.
    pub morph_targets: Vec<GltfMorphTarget>,
}

// ──────────────────────────────────────────────
// Material
// ──────────────────────────────────────────────

/// PBR material properties extracted from a glTF material.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfMaterial {
    /// Name from the glTF asset.
    pub name: String,
    /// Base colour (RGBA).
    pub base_color: [f32; 4],
    /// Path to the base-colour texture, resolved relative to the glTF file.
    pub base_color_texture: Option<String>,
    /// Metallic factor [0..1].
    pub metallic: f32,
    /// Path to the metallic-roughness texture.
    pub metallic_roughness_texture: Option<String>,
    /// Roughness factor [0..1].
    pub roughness: f32,
    /// Emissive colour (RGB).
    pub emissive: [f32; 3],
    /// Path to the emissive texture.
    pub emissive_texture: Option<String>,
    /// Path to the normal-map texture.
    pub normal_map: Option<String>,
    /// Path to the ambient-occlusion texture.
    pub occlusion_texture: Option<String>,
    /// Alpha cutoff when `alpha_mode` is `Mask`.
    pub alpha_cutoff: f32,
    /// Transparency mode.
    pub alpha_mode: GltfAlphaMode,
    /// Whether the material is double-sided.
    pub double_sided: bool,
}

/// Transparency mode, mirroring the glTF `alphaMode` property.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GltfAlphaMode {
    Opaque,
    Mask,
    Blend,
}

#[cfg(feature = "asset-pipeline")]
impl Default for GltfMaterial {
    fn default() -> Self {
        GltfMaterial {
            name: String::from("unnamed"),
            base_color: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic: 0.0,
            metallic_roughness_texture: None,
            roughness: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_texture: None,
            normal_map: None,
            occlusion_texture: None,
            alpha_cutoff: 0.5,
            alpha_mode: GltfAlphaMode::Opaque,
            double_sided: false,
        }
    }
}

// ──────────────────────────────────────────────
// Node / Transform
// ──────────────────────────────────────────────

/// A node in the glTF scene hierarchy.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfNode {
    /// Name from the glTF asset.
    pub name: String,
    /// Indices of child nodes within the `GltfScene::nodes` list.
    pub children: Vec<usize>,
    /// Index into `GltfScene::meshes`, if this node has a mesh.
    pub mesh_index: Option<usize>,
    /// Index into `GltfScene::skins`, if this node has a skin.
    pub skin_index: Option<usize>,
    /// Translation component.
    pub translation: [f32; 3],
    /// Rotation as a quaternion (xyzw).
    pub rotation: [f32; 4],
    /// Scale component.
    pub scale: [f32; 3],
}

#[cfg(feature = "asset-pipeline")]
impl Default for GltfNode {
    fn default() -> Self {
        GltfNode {
            name: String::from("unnamed"),
            children: Vec::new(),
            mesh_index: None,
            skin_index: None,
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

// ──────────────────────────────────────────────
// Animation
// ──────────────────────────────────────────────

/// Keyframe interpolation method.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GltfInterpolation {
    Linear,
    Step,
    CubicSpline,
}

/// The property that an animation channel targets.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GltfChannelValue {
    Translation(Vec<[f32; 3]>),
    Rotation(Vec<[f32; 4]>),
    Scale(Vec<[f32; 3]>),
}

/// A single animation channel: one property of one node animated over time.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfAnimationChannel {
    /// Index of the node this channel targets.
    pub node_index: usize,
    /// How keyframes are interpolated.
    pub interpolation: GltfInterpolation,
    /// Keyframe times in seconds.
    pub times: Vec<f32>,
    /// Keyframe values (translations, rotations, or scales).
    pub values: GltfChannelValue,
}

/// A named animation clip.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfAnimation {
    /// Name from the glTF asset.
    pub name: String,
    /// Duration in seconds (max keyframe time).
    pub duration: f32,
    /// Animation channels.
    pub channels: Vec<GltfAnimationChannel>,
}

// ──────────────────────────────────────────────
// Skin
// ──────────────────────────────────────────────

/// Skeletal skin data: joint indices and inverse bind matrices.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfSkin {
    /// Indices of joint nodes in `GltfScene::nodes`.
    pub joint_indices: Vec<usize>,
    /// One 4×4 inverse-bind matrix per joint.
    pub inverse_bind_matrices: Vec<[[f32; 4]; 4]>,
    /// Index of the skeleton root node (if specified).
    pub skeleton_root: Option<usize>,
}

// ──────────────────────────────────────────────
// Scene
// ──────────────────────────────────────────────

/// Top-level container for everything extracted from a single glTF file.
#[cfg(feature = "asset-pipeline")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GltfScene {
    /// All meshes.
    pub meshes: Vec<GltfMesh>,
    /// All materials.
    pub materials: Vec<GltfMaterial>,
    /// All scene-graph nodes (flat list).
    pub nodes: Vec<GltfNode>,
    /// All animation clips.
    pub animations: Vec<GltfAnimation>,
    /// All skins.
    pub skins: Vec<GltfSkin>,
    /// Indices of root-level nodes.
    pub root_nodes: Vec<usize>,
}

// ──────────────────────────────────────────────
// Importer
// ──────────────────────────────────────────────

/// Main glTF importer.
///
/// # Examples
///
/// ```ignore
/// use chronos_engine::import::gltf_import::GltfImporter;
/// let scene = GltfImporter::import("model.glb")?;
/// ```
#[cfg(feature = "asset-pipeline")]
pub struct GltfImporter;

#[cfg(feature = "asset-pipeline")]
impl GltfImporter {
    /// Import a glTF file (`.glb` or `.gltf`) from disk, returning a full
    /// [`GltfScene`] with meshes, materials, nodes, animations, and skins.
    pub fn import(path: &Path) -> Result<GltfScene, GltfImportError> {
        let (document, buffers, _images) = gltf::import(path)?;

        let base_dir = path.parent().ok_or_else(|| {
            GltfImportError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cannot resolve parent directory",
            ))
        })?;

        let materials = Self::extract_materials(&document, base_dir)?;
        let meshes = Self::extract_meshes(&document, &buffers)?;
        let (nodes, root_nodes) = Self::extract_nodes(&document);
        let animations = Self::extract_animations(&document, &buffers)?;
        let skins = Self::extract_skins(&document, &buffers);

        Ok(GltfScene {
            meshes,
            materials,
            nodes,
            animations,
            skins,
            root_nodes,
        })
    }

    /// Import only meshes from a glTF file (lightweight mode).
    /// Skips materials, animations, skins, and node hierarchy.
    pub fn import_meshes_only(path: &Path) -> Result<Vec<GltfMesh>, GltfImportError> {
        let (document, buffers, _images) = gltf::import(path)?;
        Self::extract_meshes(&document, &buffers)
    }

    // ── Mesh extraction ──────────────────────

    fn extract_meshes(
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
    ) -> Result<Vec<GltfMesh>, GltfImportError> {
        let mut meshes = Vec::new();
        for gltf_mesh in document.meshes() {
            let mesh_name = gltf_mesh.name().unwrap_or("unnamed").to_string();

            // Collect all primitives into one mesh.
            // We merge by offsetting indices per primitive.
            let mut all_vertices: Vec<GltfVertex> = Vec::new();
            let mut all_indices: Vec<u32> = Vec::new();
            let mut material_index: Option<usize> = None;
            let mut morph_targets: Vec<GltfMorphTarget> = Vec::new();

            for prim in gltf_mesh.primitives() {
                let mode = prim.mode();
                if mode != gltf::mesh::Mode::Triangles {
                    return Err(GltfImportError::UnsupportedFeature(format!(
                        "primitive mode {:?} is not supported (only Triangles)",
                        mode
                    )));
                }

                let reader = prim.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or_else(|| {
                        GltfImportError::InvalidMesh(format!(
                            "mesh '{}' primitive has no POSITION attribute",
                            mesh_name
                        ))
                    })?
                    .collect();

                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|n| n.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

                let tex_coords: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|tc| tc.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                let vertex_count = positions.len();
                let base = all_vertices.len() as u32;

                for i in 0..vertex_count {
                    all_vertices.push(GltfVertex::new(positions[i], normals[i], tex_coords[i]));
                }

                // Read indices (generate sequential if absent).
                if let Some(read_indices) = reader.read_indices() {
                    let idx: Vec<u32> = read_indices.into_u32().map(|i| base + i).collect();
                    all_indices.extend(idx);
                } else {
                    for i in 0..vertex_count as u32 {
                        all_indices.push(base + i);
                    }
                }

                // Capture material index from the first primitive that has one.
                if material_index.is_none() {
                    material_index = prim.material().index();
                }

                // Morph targets
                let morph_reader = reader.read_morph_targets();
                for (pos_d, norm_d, tang_d) in morph_reader {
                    morph_targets.push(GltfMorphTarget {
                        position_deltas: pos_d.map(|i| i.collect()).unwrap_or_default(),
                        normal_deltas: norm_d.map(|i| i.collect()).unwrap_or_default(),
                        tangent_deltas: tang_d.map(|i| i.collect()).unwrap_or_default(),
                    });
                }
            }

            meshes.push(GltfMesh {
                name: mesh_name,
                vertices: all_vertices,
                indices: all_indices,
                material_index,
                morph_targets,
            });
        }
        Ok(meshes)
    }

    // ── Material extraction ──────────────────

    fn extract_materials(
        document: &gltf::Document,
        base_dir: &Path,
    ) -> Result<Vec<GltfMaterial>, GltfImportError> {
        let mut materials = Vec::new();

        // Always add a default material at index 0 for primitives without one.
        materials.push(GltfMaterial::default());

        for mat in document.materials() {
            let pbr = mat.pbr_metallic_roughness();

            let base_color_texture = pbr
                .base_color_texture()
                .map(|info| Self::resolve_texture_path(&info.texture(), base_dir))
                .transpose()?;

            let metallic_roughness_texture = pbr
                .metallic_roughness_texture()
                .map(|info| Self::resolve_texture_path(&info.texture(), base_dir))
                .transpose()?;

            let normal_map = mat
                .normal_texture()
                .map(|info| Self::resolve_texture_path(&info.texture(), base_dir))
                .transpose()?;

            let occlusion_texture = mat
                .occlusion_texture()
                .map(|info| Self::resolve_texture_path(&info.texture(), base_dir))
                .transpose()?;

            let emissive_texture = mat
                .emissive_texture()
                .map(|info| Self::resolve_texture_path(&info.texture(), base_dir))
                .transpose()?;

            let alpha_mode = match mat.alpha_mode() {
                gltf::material::AlphaMode::Opaque => GltfAlphaMode::Opaque,
                gltf::material::AlphaMode::Mask => GltfAlphaMode::Mask,
                gltf::material::AlphaMode::Blend => GltfAlphaMode::Blend,
            };

            materials.push(GltfMaterial {
                name: mat.name().unwrap_or("unnamed").to_string(),
                base_color: pbr.base_color_factor(),
                base_color_texture,
                metallic: pbr.metallic_factor(),
                metallic_roughness_texture,
                roughness: pbr.roughness_factor(),
                emissive: mat.emissive_factor(),
                emissive_texture,
                normal_map,
                occlusion_texture,
                alpha_cutoff: mat.alpha_cutoff().unwrap_or(0.5),
                alpha_mode,
                double_sided: mat.double_sided(),
            });
        }
        Ok(materials)
    }

    /// Resolve a glTF texture reference to a filesystem path string.
    fn resolve_texture_path(
        texture: &gltf::Texture<'_>,
        base_dir: &Path,
    ) -> Result<String, GltfImportError> {
        match texture.source().source() {
            gltf::image::Source::Uri { uri, .. } => {
                let path = base_dir.join(uri);
                Ok(path.to_string_lossy().to_string())
            }
            gltf::image::Source::View { .. } => {
                // Embedded image data — no file path to resolve.
                Ok(String::new())
            }
        }
    }

    // ── Node extraction ──────────────────────

    fn extract_nodes(document: &gltf::Document) -> (Vec<GltfNode>, Vec<usize>) {
        let mut nodes = Vec::new();

        for node in document.nodes() {
            let (translation, rotation, scale) = node.transform().decomposed();

            nodes.push(GltfNode {
                name: node.name().unwrap_or("unnamed").to_string(),
                children: node.children().map(|c| c.index()).collect(),
                mesh_index: node.mesh().map(|m| m.index()),
                skin_index: node.skin().map(|s| s.index()),
                translation,
                rotation,
                scale,
            });
        }

        // Collect root nodes from the default scene (or all scenes).
        let root_nodes: Vec<usize> = document
            .default_scene()
            .map(|scene| scene.nodes().map(|n| n.index()).collect())
            .unwrap_or_default();

        (nodes, root_nodes)
    }

    // ── Animation extraction ─────────────────

    fn extract_animations(
        document: &gltf::Document,
        buffers: &[gltf::buffer::Data],
    ) -> Result<Vec<GltfAnimation>, GltfImportError> {
        let mut animations = Vec::new();

        for anim in document.animations() {
            let name = anim.name().unwrap_or("unnamed").to_string();
            let mut channels = Vec::new();
            let mut max_time = 0.0_f32;

            for channel in anim.channels() {
                let reader = channel.reader(|buffer| Some(&buffers[buffer.index()]));

                let times: Vec<f32> = reader
                    .read_inputs()
                    .ok_or_else(|| {
                        GltfImportError::Parse(format!(
                            "animation '{}' channel has no input accessor",
                            name
                        ))
                    })?
                    .collect();

                if let Some(&t) = times.last() {
                    max_time = max_time.max(t);
                }

                let interpolation = match channel.sampler().interpolation() {
                    gltf::animation::Interpolation::Linear => GltfInterpolation::Linear,
                    gltf::animation::Interpolation::Step => GltfInterpolation::Step,
                    gltf::animation::Interpolation::CubicSpline => GltfInterpolation::CubicSpline,
                };

                let node_index = channel.target().node().index();

                let values = match reader.read_outputs() {
                    Some(gltf::animation::util::ReadOutputs::Translations(iter)) => {
                        GltfChannelValue::Translation(iter.collect())
                    }
                    Some(gltf::animation::util::ReadOutputs::Rotations(rotations)) => {
                        GltfChannelValue::Rotation(rotations.into_f32().collect())
                    }
                    Some(gltf::animation::util::ReadOutputs::Scales(iter)) => {
                        GltfChannelValue::Scale(iter.collect())
                    }
                    Some(gltf::animation::util::ReadOutputs::MorphTargetWeights(_)) => {
                        // Morph-target weights are not tracked as animation channels here.
                        continue;
                    }
                    None => {
                        return Err(GltfImportError::Parse(format!(
                            "animation '{}' channel has no output accessor",
                            name
                        )));
                    }
                };

                channels.push(GltfAnimationChannel {
                    node_index,
                    interpolation,
                    times,
                    values,
                });
            }

            animations.push(GltfAnimation {
                name,
                duration: max_time,
                channels,
            });
        }

        Ok(animations)
    }

    // ── Skin extraction ──────────────────────

    fn extract_skins(document: &gltf::Document, buffers: &[gltf::buffer::Data]) -> Vec<GltfSkin> {
        let mut skins = Vec::new();

        for skin in document.skins() {
            let joint_indices: Vec<usize> = skin.joints().map(|j| j.index()).collect();

            let inverse_bind_matrices = skin
                .reader(|buffer| Some(&buffers[buffer.index()]))
                .read_inverse_bind_matrices()
                .map(|iter| iter.collect())
                .unwrap_or_else(|| {
                    // Default to identity matrices when IBMs are absent.
                    vec![
                        [
                            [1.0, 0.0, 0.0, 0.0],
                            [0.0, 1.0, 0.0, 0.0],
                            [0.0, 0.0, 1.0, 0.0],
                            [0.0, 0.0, 0.0, 1.0]
                        ];
                        joint_indices.len()
                    ]
                });

            let skeleton_root = skin.skeleton().map(|n| n.index());

            skins.push(GltfSkin {
                joint_indices,
                inverse_bind_matrices,
                skeleton_root,
            });
        }

        skins
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(all(test, feature = "asset-pipeline"))]
mod tests {
    use super::*;
    use std::fs;

    const TEST_BASE: &str = "/tmp/chronos_import_tests/gltf";

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from(TEST_BASE).join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("failed to create test dir");
        dir
    }

    /// Write raw f32 bytes to a .bin file, return its file name.
    fn write_bin(dir: &Path, name: &str, data: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, data).expect("failed to write bin");
        path
    }

    /// Build a minimal gltf JSON string referencing `bin_name` with one
    /// triangle (3 vertices). Returns (json, bytes_written).
    fn triangle_gltf_json(bin_name: &str, byte_length: usize) -> String {
        format!(
            r#"{{
                "asset": {{"version": "2.0", "generator": "chronos-test"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"name": "TriNode", "mesh": 0}}],
                "meshes": [{{"name": "TriMesh", "primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "accessors": [{{
                    "bufferView": 0,
                    "componentType": 5126,
                    "count": 3,
                    "type": "VEC3",
                    "max": [1.0, 1.0, 0.0],
                    "min": [-1.0, -1.0, 0.0]
                }}],
                "bufferViews": [{{
                    "buffer": 0,
                    "byteOffset": 0,
                    "byteLength": {byte_length}
                }}],
                "buffers": [{{"uri": "{bin_name}", "byteLength": {byte_length}}}]
            }}"#
        )
    }

    /// Build a gltf JSON with positions + normals + UVs.
    fn full_vertex_gltf_json(bin_name: &str, byte_length: usize) -> String {
        format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"mesh": 0}}],
                "meshes": [{{"primitives": [{{"attributes": {{
                    "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2
                }}}}]}}],
                "accessors": [
                    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}},
                    {{"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3", "max": [0,1,0], "min": [0,0,0]}},
                    {{"bufferView": 2, "componentType": 5126, "count": 3, "type": "VEC2", "max": [1,1], "min": [0,0]}}
                ],
                "bufferViews": [
                    {{"buffer": 0, "byteOffset": 0,  "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 36, "byteLength": 36}},
                    {{"buffer": 0, "byteOffset": 72, "byteLength": 24}}
                ],
                "buffers": [{{"uri": "{bin_name}", "byteLength": {byte_length}}}]
            }}"#
        )
    }

    fn f32_bytes(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    // ── 1. Import minimal valid .gltf ────────

    #[test]
    fn test_import_minimal_gltf() {
        let dir = setup("minimal_gltf");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "tri.bin", &bytes);
        let json = triangle_gltf_json("tri.bin", bytes.len());
        let gltf_path = dir.join("minimal.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].vertices.len(), 3);
        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.root_nodes, vec![0]);
    }

    // ── 2. Import minimal .glb ──────────────

    #[test]
    fn test_import_minimal_glb() {
        let dir = setup("minimal_glb");

        // Build a valid GLB: header + JSON chunk + BIN chunk
        let json = r#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","max":[1,1,0],"min":[0,0,0]}],"bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],"buffers":[{"byteLength":36}]}"#;

        // Pad JSON to 4-byte alignment
        let json_bytes = json.as_bytes();
        let json_pad = (4 - json_bytes.len() % 4) % 4;
        let json_len = json_bytes.len() + json_pad;
        let mut json_padded = json_bytes.to_vec();
        json_padded.extend(std::iter::repeat(0x20u8).take(json_pad));

        // Binary data: 3 vertices * 3 floats = 36 bytes
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bin_data = f32_bytes(pos);
        let bin_pad = (4 - bin_data.len() % 4) % 4;
        let bin_len = bin_data.len() + bin_pad;
        let mut bin_padded = bin_data.clone();
        bin_padded.extend(std::iter::repeat(0u8).take(bin_pad));

        let total = 12 + 8 + json_len + 8 + bin_len;
        let mut glb = Vec::with_capacity(total);

        // Header
        glb.extend(b"glTF"); // magic
        glb.extend(&2u32.to_le_bytes()); // version
        glb.extend(&(total as u32).to_le_bytes()); // total length
                                                   // JSON chunk
        glb.extend(&(json_len as u32).to_le_bytes());
        glb.extend(b"JSON");
        glb.extend(&json_padded);
        // BIN chunk
        glb.extend(&(bin_len as u32).to_le_bytes());
        glb.extend(b"BIN\0");
        glb.extend(&bin_padded);

        let glb_path = dir.join("minimal.glb");
        fs::write(&glb_path, &glb).unwrap();

        let scene = GltfImporter::import(&glb_path).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.meshes[0].vertices.len(), 3);
        // Check vertex positions
        let v = &scene.meshes[0].vertices[1];
        assert!((v.x - 1.0).abs() < 0.001);
    }

    // ── 3. Parse mesh with positions only ────

    #[test]
    fn test_mesh_positions_only() {
        let dir = setup("pos_only");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "pos.bin", &bytes);
        let json = triangle_gltf_json("pos.bin", bytes.len());
        let gltf_path = dir.join("pos.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        let mesh = &scene.meshes[0];
        assert_eq!(mesh.vertices.len(), 3);
        // Normals should default to (0, 1, 0)
        for v in &mesh.vertices {
            assert!((v.ny - 1.0).abs() < 0.001);
            assert!((v.u - 0.0).abs() < 0.001);
        }
    }

    // ── 4. Parse mesh with positions + normals + UVs ──

    #[test]
    fn test_mesh_full_vertex() {
        let dir = setup("full_vertex");
        // 3 positions (3×f32×3 = 36 bytes), 3 normals (36), 3 UVs (3×f32×2 = 24)
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let norms: &[f32] = &[0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0];
        let uvs: &[f32] = &[0.0, 0.0, 1.0, 0.0, 0.0, 1.0];
        let mut bytes = f32_bytes(pos);
        bytes.extend(f32_bytes(norms));
        bytes.extend(f32_bytes(uvs));
        write_bin(&dir, "full.bin", &bytes);
        let json = full_vertex_gltf_json("full.bin", bytes.len());
        let gltf_path = dir.join("full.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        let mesh = &scene.meshes[0];
        assert_eq!(mesh.vertices.len(), 3);

        // Second vertex: pos (1,0,0), norm (0,1,0), uv (1,0)
        let v = &mesh.vertices[1];
        assert!((v.x - 1.0).abs() < 0.001);
        assert!((v.ny - 1.0).abs() < 0.001);
        assert!((v.u - 1.0).abs() < 0.001);
    }

    // ── 5. Parse material with PBR properties ──

    #[test]
    fn test_material_pbr() {
        let dir = setup("mat_pbr");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "mat.bin", &bytes);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"mesh": 0}}],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}, "material": 0}}]}}],
                "materials": [{{
                    "name": "TestPBR",
                    "pbrMetallicRoughness": {{
                        "baseColorFactor": [0.8, 0.2, 0.1, 1.0],
                        "metallicFactor": 0.9,
                        "roughnessFactor": 0.3
                    }},
                    "emissiveFactor": [1.0, 0.5, 0.0],
                    "alphaMode": "MASK",
                    "alphaCutoff": 0.75,
                    "doubleSided": true
                }}],
                "accessors": [{{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}}],
                "bufferViews": [{{"buffer": 0, "byteOffset": 0, "byteLength": {}}}],
                "buffers": [{{"uri": "mat.bin", "byteLength": {}}}]
            }}"#,
            bytes.len(),
            bytes.len()
        );
        let gltf_path = dir.join("mat.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        // materials[0] is the default, materials[1] is ours
        assert!(scene.materials.len() >= 2);
        let mat = scene
            .materials
            .iter()
            .find(|m| m.name == "TestPBR")
            .unwrap();
        assert!((mat.base_color[0] - 0.8).abs() < 0.01);
        assert!((mat.metallic - 0.9).abs() < 0.01);
        assert!((mat.roughness - 0.3).abs() < 0.01);
        assert!((mat.emissive[0] - 1.0).abs() < 0.01);
        assert_eq!(mat.alpha_mode, GltfAlphaMode::Mask);
        assert!((mat.alpha_cutoff - 0.75).abs() < 0.01);
        assert!(mat.double_sided);
    }

    // ── 6. Parse scene node hierarchy ────────

    #[test]
    fn test_node_hierarchy() {
        let dir = setup("hierarchy");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "hier.bin", &bytes);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [
                    {{"name": "Root", "children": [1], "mesh": 0}},
                    {{"name": "Child", "translation": [1.0, 2.0, 3.0]}}
                ],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "accessors": [{{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}}],
                "bufferViews": [{{"buffer": 0, "byteOffset": 0, "byteLength": {}}}],
                "buffers": [{{"uri": "hier.bin", "byteLength": {}}}]
            }}"#,
            bytes.len(),
            bytes.len()
        );
        let gltf_path = dir.join("hier.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        assert_eq!(scene.nodes.len(), 2);
        let root = &scene.nodes[0];
        assert_eq!(root.name, "Root");
        assert_eq!(root.children, vec![1]);
        assert_eq!(root.mesh_index, Some(0));

        let child = &scene.nodes[1];
        assert_eq!(child.name, "Child");
        assert!((child.translation[0] - 1.0).abs() < 0.001);
        assert!((child.translation[1] - 2.0).abs() < 0.001);
        assert!((child.translation[2] - 3.0).abs() < 0.001);
    }

    // ── 7. Parse animation channels ──────────

    #[test]
    fn test_animation_channels() {
        let dir = setup("animation");

        // Animation data: 2 keyframe times + 2 translations
        let times: &[f32] = &[0.0, 1.0];
        let translations: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let mut buf = f32_bytes(times);
        buf.extend(f32_bytes(translations));
        write_bin(&dir, "anim.bin", &buf);

        // Also need a mesh so the scene is valid
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let pos_bytes = f32_bytes(pos);
        write_bin(&dir, "amesh.bin", &pos_bytes);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{"name": "Animated", "mesh": 0}}],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "accessors": [
                    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}},
                    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "SCALAR", "max": [1], "min": [0]}},
                    {{"bufferView": 2, "componentType": 5126, "count": 2, "type": "VEC3"}}
                ],
                "bufferViews": [
                    {{"buffer": 0, "byteOffset": 0, "byteLength": {pos_len}}},
                    {{"buffer": 1, "byteOffset": 0, "byteLength": 8}},
                    {{"buffer": 1, "byteOffset": 8, "byteLength": 24}}
                ],
                "buffers": [
                    {{"uri": "amesh.bin", "byteLength": {pos_len}}},
                    {{"uri": "anim.bin", "byteLength": {anim_len}}}
                ],
                "animations": [{{
                    "name": "Slide",
                    "channels": [{{
                        "sampler": 0,
                        "target": {{"node": 0, "path": "translation"}}
                    }}],
                    "samplers": [{{
                        "input": 1,
                        "output": 2,
                        "interpolation": "LINEAR"
                    }}]
                }}]
            }}"#,
            pos_len = pos_bytes.len(),
            anim_len = buf.len(),
        );
        let gltf_path = dir.join("anim.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        assert_eq!(scene.animations.len(), 1);
        let anim = &scene.animations[0];
        assert_eq!(anim.name, "Slide");
        assert!((anim.duration - 1.0).abs() < 0.001);
        assert_eq!(anim.channels.len(), 1);

        let ch = &anim.channels[0];
        assert_eq!(ch.node_index, 0);
        assert_eq!(ch.interpolation, GltfInterpolation::Linear);
        assert_eq!(ch.times.len(), 2);

        if let GltfChannelValue::Translation(ref vals) = ch.values {
            assert_eq!(vals.len(), 2);
            assert!((vals[1][0] - 1.0).abs() < 0.001);
        } else {
            panic!("expected Translation channel");
        }
    }

    // ── 8. Parse skin with joints ────────────

    #[test]
    fn test_skin_with_joints() {
        let dir = setup("skin");

        // 2 identity inverse bind matrices (each 16 floats = 64 bytes)
        let identity: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let mut ibm_bytes = Vec::new();
        for _ in 0..2 {
            ibm_bytes.extend(identity.iter().flat_map(|f| f.to_le_bytes()));
        }
        write_bin(&dir, "skin.bin", &ibm_bytes);

        // A mesh with skinning
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let pos_bytes = f32_bytes(pos);
        write_bin(&dir, "smesh.bin", &pos_bytes);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0, 1]}}],
                "nodes": [
                    {{"name": "Armature", "children": [2]}},
                    {{"name": "SkinnedMesh", "mesh": 0, "skin": 0}},
                    {{"name": "Joint1"}}
                ],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "skins": [{{
                    "joints": [0, 2],
                    "inverseBindMatrices": 1,
                    "skeleton": 0
                }}],
                "accessors": [
                    {{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}},
                    {{"bufferView": 1, "componentType": 5126, "count": 2, "type": "MAT4"}}
                ],
                "bufferViews": [
                    {{"buffer": 0, "byteOffset": 0, "byteLength": {pos_len}}},
                    {{"buffer": 1, "byteOffset": 0, "byteLength": {ibm_len}}}
                ],
                "buffers": [
                    {{"uri": "smesh.bin", "byteLength": {pos_len}}},
                    {{"uri": "skin.bin", "byteLength": {ibm_len}}}
                ]
            }}"#,
            pos_len = pos_bytes.len(),
            ibm_len = ibm_bytes.len(),
        );
        let gltf_path = dir.join("skin.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        assert_eq!(scene.skins.len(), 1);
        let skin = &scene.skins[0];
        assert_eq!(skin.joint_indices.len(), 2);
        assert_eq!(skin.joint_indices, vec![0, 2]);
        assert_eq!(skin.inverse_bind_matrices.len(), 2);
        assert_eq!(skin.skeleton_root, Some(0));
        // Identity matrix check
        assert!((skin.inverse_bind_matrices[0][0][0] - 1.0).abs() < 0.001);
    }

    // ── 9. Error on nonexistent file ─────────

    #[test]
    fn test_error_nonexistent_file() {
        let result =
            GltfImporter::import(Path::new("/tmp/chronos_import_tests/does_not_exist.gltf"));
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("glTF")
                || msg.contains("I/O")
                || msg.contains("parse")
                || msg.contains("No such")
        );
    }

    // ── 10. Error on invalid glTF content ────

    #[test]
    fn test_error_invalid_content() {
        let dir = setup("invalid");
        let bad_path = dir.join("bad.gltf");
        fs::write(&bad_path, "this is not valid gltf at all").unwrap();
        let result = GltfImporter::import(&bad_path);
        assert!(result.is_err());
    }

    // ── 11. Extract TRS transform from node ──

    #[test]
    fn test_node_trs_transform() {
        let dir = setup("trs");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "trs.bin", &bytes);

        let json = format!(
            r#"{{
                "asset": {{"version": "2.0"}},
                "scene": 0,
                "scenes": [{{"nodes": [0]}}],
                "nodes": [{{
                    "name": "TRSNode",
                    "translation": [10.0, 20.0, 30.0],
                    "rotation": [0.0, 0.0, 0.7071, 0.7071],
                    "scale": [2.0, 3.0, 4.0],
                    "mesh": 0
                }}],
                "meshes": [{{"primitives": [{{"attributes": {{"POSITION": 0}}}}]}}],
                "accessors": [{{"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "max": [1,1,0], "min": [0,0,0]}}],
                "bufferViews": [{{"buffer": 0, "byteOffset": 0, "byteLength": {}}}],
                "buffers": [{{"uri": "trs.bin", "byteLength": {}}}]
            }}"#,
            bytes.len(),
            bytes.len()
        );
        let gltf_path = dir.join("trs.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let scene = GltfImporter::import(&gltf_path).unwrap();
        let node = &scene.nodes[0];
        assert!((node.translation[0] - 10.0).abs() < 0.01);
        assert!((node.translation[1] - 20.0).abs() < 0.01);
        assert!((node.translation[2] - 30.0).abs() < 0.01);
        assert!((node.rotation[2] - 0.7071).abs() < 0.01);
        assert!((node.rotation[3] - 0.7071).abs() < 0.01);
        assert!((node.scale[0] - 2.0).abs() < 0.01);
        assert!((node.scale[1] - 3.0).abs() < 0.01);
        assert!((node.scale[2] - 4.0).abs() < 0.01);
    }

    // ── 12. import_meshes_only skips rest ────

    #[test]
    fn test_meshes_only_skips_extras() {
        let dir = setup("meshes_only");
        let pos: &[f32] = &[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        let bytes = f32_bytes(pos);
        write_bin(&dir, "mo.bin", &bytes);

        let json = triangle_gltf_json("mo.bin", bytes.len());
        let gltf_path = dir.join("mo.gltf");
        fs::write(&gltf_path, &json).unwrap();

        let meshes = GltfImporter::import_meshes_only(&gltf_path).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 3);
        // No scene/node data returned
    }

    // ── 13. Default GltfMaterial values ──────

    #[test]
    fn test_default_material() {
        let mat = GltfMaterial::default();
        assert_eq!(mat.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 1.0);
        assert_eq!(mat.emissive, [0.0, 0.0, 0.0]);
        assert_eq!(mat.alpha_mode, GltfAlphaMode::Opaque);
        assert!(mat.normal_map.is_none());
        assert!(mat.occlusion_texture.is_none());
    }

    // ── 14. Default GltfNode values ──────────

    #[test]
    fn test_default_node() {
        let node = GltfNode::default();
        assert_eq!(node.translation, [0.0, 0.0, 0.0]);
        assert_eq!(node.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(node.scale, [1.0, 1.0, 1.0]);
        assert!(node.children.is_empty());
        assert!(node.mesh_index.is_none());
        assert!(node.skin_index.is_none());
    }

    // ── 15. Error types implement Display ────

    #[test]
    fn test_error_display() {
        let err = GltfImportError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(err.to_string().contains("gone"));

        let err = GltfImportError::Parse("bad json".into());
        assert!(err.to_string().contains("bad json"));

        let err = GltfImportError::InvalidMesh("no verts".into());
        assert!(err.to_string().contains("no verts"));

        let err = GltfImportError::InvalidMaterial("no base".into());
        assert!(err.to_string().contains("no base"));

        let err = GltfImportError::MissingBuffer("buf0".into());
        assert!(err.to_string().contains("buf0"));

        let err = GltfImportError::UnsupportedFeature("draco".into());
        assert!(err.to_string().contains("draco"));
    }

    // ── 16. Serialization round-trip ─────────

    #[test]
    fn test_serialization_roundtrip() {
        let scene = GltfScene {
            meshes: vec![GltfMesh {
                name: "Tri".into(),
                vertices: vec![GltfVertex::new(
                    [1.0, 2.0, 3.0],
                    [0.0, 1.0, 0.0],
                    [0.5, 0.5],
                )],
                indices: vec![0],
                material_index: Some(0),
                morph_targets: vec![],
            }],
            materials: vec![GltfMaterial::default()],
            nodes: vec![GltfNode::default()],
            animations: vec![],
            skins: vec![],
            root_nodes: vec![0],
        };

        let json = serde_json::to_string(&scene).unwrap();
        let parsed: GltfScene = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.meshes.len(), 1);
        assert!((parsed.meshes[0].vertices[0].x - 1.0).abs() < 0.001);
        assert_eq!(parsed.root_nodes, vec![0]);
    }
}
