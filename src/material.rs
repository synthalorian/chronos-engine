//! Material definition system for Chronos Engine.
//!
//! Provides material properties, render state configuration, built-in presets,
//! and compilation to GPU-ready uniform data.

use std::fmt;

// ---------------------------------------------------------------------------
// MaterialValue
// ---------------------------------------------------------------------------

/// A typed value that can be assigned to a material property.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
    Texture(String),
    Bool(bool),
    Int(i32),
}

impl MaterialValue {
    /// Returns the size in bytes needed to pack this value into a uniform buffer.
    pub fn uniform_size(&self) -> usize {
        match self {
            MaterialValue::Float(_) => 4,
            MaterialValue::Vec2(_) => 8,
            MaterialValue::Vec3(_) => 12,
            MaterialValue::Vec4(_) => 16,
            MaterialValue::Color(_) => 16,
            MaterialValue::Texture(_) => 0, // textures are bound separately
            MaterialValue::Bool(_) => 4,
            MaterialValue::Int(_) => 4,
        }
    }

    /// Write this value into a byte buffer for GPU upload.
    pub fn write_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            MaterialValue::Float(v) => buf.extend_from_slice(&v.to_le_bytes()),
            MaterialValue::Vec2(v) => {
                buf.extend_from_slice(&v[0].to_le_bytes());
                buf.extend_from_slice(&v[1].to_le_bytes());
            }
            MaterialValue::Vec3(v) => {
                buf.extend_from_slice(&v[0].to_le_bytes());
                buf.extend_from_slice(&v[1].to_le_bytes());
                buf.extend_from_slice(&v[2].to_le_bytes());
            }
            MaterialValue::Vec4(v) | MaterialValue::Color(v) => {
                for c in v {
                    buf.extend_from_slice(&c.to_le_bytes());
                }
            }
            MaterialValue::Texture(_) => {}
            MaterialValue::Bool(v) => {
                let int: u32 = if *v { 1 } else { 0 };
                buf.extend_from_slice(&int.to_le_bytes());
            }
            MaterialValue::Int(v) => buf.extend_from_slice(&v.to_le_bytes()),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            MaterialValue::Float(_) => "Float",
            MaterialValue::Vec2(_) => "Vec2",
            MaterialValue::Vec3(_) => "Vec3",
            MaterialValue::Vec4(_) => "Vec4",
            MaterialValue::Color(_) => "Color",
            MaterialValue::Texture(_) => "Texture",
            MaterialValue::Bool(_) => "Bool",
            MaterialValue::Int(_) => "Int",
        }
    }
}

// ---------------------------------------------------------------------------
// BlendMode / CullMode / RenderState
// ---------------------------------------------------------------------------

/// How the material's fragments blend with the framebuffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    AlphaBlend,
    Additive,
    Multiply,
}

/// Which triangle faces to cull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CullMode {
    None,
    Front,
    #[default]
    Back,
}

/// GPU render-state knobs attached to a material.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderState {
    pub blend_mode: BlendMode,
    pub cull_mode: CullMode,
    pub depth_write: bool,
    pub depth_test: bool,
    pub wireframe: bool,
}

impl Default for RenderState {
    fn default() -> Self {
        RenderState {
            blend_mode: BlendMode::Opaque,
            cull_mode: CullMode::Back,
            depth_write: true,
            depth_test: true,
            wireframe: false,
        }
    }
}

// ---------------------------------------------------------------------------
// MaterialProperty
// ---------------------------------------------------------------------------

/// A single named property with a current value and a default fallback.
#[derive(Debug, Clone)]
pub struct MaterialProperty {
    pub name: String,
    pub value: MaterialValue,
    pub default: MaterialValue,
}

impl MaterialProperty {
    pub fn new(name: &str, value: MaterialValue) -> Self {
        let default = value.clone();
        MaterialProperty {
            name: name.to_string(),
            value,
            default,
        }
    }

    pub fn with_default(mut self, default: MaterialValue) -> Self {
        self.default = default;
        self
    }
}

// ---------------------------------------------------------------------------
// MaterialError
// ---------------------------------------------------------------------------

/// Errors that can arise when working with materials.
#[derive(Debug, Clone, PartialEq)]
pub enum MaterialError {
    PropertyNotFound(String),
    TypeMismatch(String),
    CompilationFailed(String),
}

impl fmt::Display for MaterialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaterialError::PropertyNotFound(name) => {
                write!(f, "material property not found: {}", name)
            }
            MaterialError::TypeMismatch(msg) => {
                write!(f, "material type mismatch: {}", msg)
            }
            MaterialError::CompilationFailed(msg) => {
                write!(f, "material compilation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for MaterialError {}

// ---------------------------------------------------------------------------
// CompiledMaterial
// ---------------------------------------------------------------------------

/// A material that has been packed and is ready for GPU submission.
#[derive(Debug, Clone)]
pub struct CompiledMaterial {
    pub shader_name: String,
    pub uniform_data: Vec<u8>,
    pub texture_paths: Vec<String>,
    pub render_state: RenderState,
}

// ---------------------------------------------------------------------------
// MaterialDefinition
// ---------------------------------------------------------------------------

/// Full material definition including properties and render state.
#[derive(Debug, Clone)]
pub struct MaterialDefinition {
    pub name: String,
    pub shader_name: String,
    pub properties: Vec<MaterialProperty>,
    pub render_state: RenderState,
}

impl MaterialDefinition {
    /// Create a new material definition targeting the given shader.
    pub fn new(name: &str, shader: &str) -> Self {
        MaterialDefinition {
            name: name.to_string(),
            shader_name: shader.to_string(),
            properties: Vec::new(),
            render_state: RenderState::default(),
        }
    }

    /// Add an arbitrary property.
    pub fn with_property(mut self, prop: MaterialProperty) -> Self {
        self.properties.push(prop);
        self
    }

    /// Convenience: set albedo color (RGBA).
    pub fn with_albedo(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.properties.push(MaterialProperty::new(
            "albedo",
            MaterialValue::Color([r, g, b, a]),
        ));
        self
    }

    /// Convenience: set normal-map texture path.
    pub fn with_normal(mut self, path: &str) -> Self {
        self.properties.push(MaterialProperty::new(
            "normal_map",
            MaterialValue::Texture(path.to_string()),
        ));
        self
    }

    /// Convenience: set metallic factor.
    pub fn with_metallic(mut self, value: f32) -> Self {
        self.properties.push(MaterialProperty::new(
            "metallic",
            MaterialValue::Float(value),
        ));
        self
    }

    /// Convenience: set roughness factor.
    pub fn with_roughness(mut self, value: f32) -> Self {
        self.properties.push(MaterialProperty::new(
            "roughness",
            MaterialValue::Float(value),
        ));
        self
    }

    /// Convenience: set emissive color.
    pub fn with_emissive(mut self, r: f32, g: f32, b: f32) -> Self {
        self.properties.push(MaterialProperty::new(
            "emissive",
            MaterialValue::Vec3([r, g, b]),
        ));
        self
    }

    /// Convenience: set albedo texture.
    pub fn with_albedo_texture(mut self, path: &str) -> Self {
        self.properties.push(MaterialProperty::new(
            "albedo_map",
            MaterialValue::Texture(path.to_string()),
        ));
        self
    }

    /// Convenience: set opacity.
    pub fn with_opacity(mut self, value: f32) -> Self {
        self.properties.push(MaterialProperty::new(
            "opacity",
            MaterialValue::Float(value),
        ));
        self
    }

    /// Look up a property value by name.
    pub fn get_property(&self, name: &str) -> Option<&MaterialValue> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| &p.value)
    }

    /// Set a property value by name. Returns an error if the property doesn't
    /// exist or if the new value's type doesn't match the existing one.
    pub fn set_property(&mut self, name: &str, value: MaterialValue) -> Result<(), MaterialError> {
        let prop = self
            .properties
            .iter_mut()
            .find(|p| p.name == name)
            .ok_or_else(|| MaterialError::PropertyNotFound(name.to_string()))?;

        if std::mem::discriminant(&prop.value) != std::mem::discriminant(&value) {
            return Err(MaterialError::TypeMismatch(format!(
                "expected {}, got {}",
                prop.value.type_name(),
                value.type_name(),
            )));
        }

        prop.value = value;
        Ok(())
    }

    /// Compile this material into a GPU-ready packed representation.
    pub fn compile(&self) -> CompiledMaterial {
        let mut uniform_data = Vec::new();
        let mut texture_paths = Vec::new();

        for prop in &self.properties {
            match &prop.value {
                MaterialValue::Texture(path) => {
                    texture_paths.push(path.clone());
                }
                other => {
                    other.write_bytes(&mut uniform_data);
                }
            }
        }

        // Align to 16-byte boundary for GPU uniform buffers.
        while uniform_data.len() % 16 != 0 {
            uniform_data.push(0);
        }

        CompiledMaterial {
            shader_name: self.shader_name.clone(),
            uniform_data,
            texture_paths,
            render_state: self.render_state,
        }
    }

    /// Override render state.
    pub fn with_render_state(mut self, state: RenderState) -> Self {
        self.render_state = state;
        self
    }

    /// Shortcut: set blend mode.
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.render_state.blend_mode = mode;
        self
    }

    /// Shortcut: set cull mode.
    pub fn with_cull_mode(mut self, mode: CullMode) -> Self {
        self.render_state.cull_mode = mode;
        self
    }

    /// Shortcut: toggle wireframe.
    pub fn with_wireframe(mut self, on: bool) -> Self {
        self.render_state.wireframe = on;
        self
    }
}

// ---------------------------------------------------------------------------
// Built-in material presets
// ---------------------------------------------------------------------------

/// Simple unlit colour material — no lighting calculations.
pub fn unlit() -> MaterialDefinition {
    MaterialDefinition::new("unlit", "unlit")
        .with_albedo(1.0, 1.0, 1.0, 1.0)
        .with_render_state(RenderState {
            blend_mode: BlendMode::Opaque,
            cull_mode: CullMode::Back,
            depth_write: true,
            depth_test: true,
            wireframe: false,
        })
}

/// Standard PBR material with albedo / metallic / roughness / normal.
pub fn pbr_standard() -> MaterialDefinition {
    MaterialDefinition::new("pbr_standard", "pbr")
        .with_albedo(0.8, 0.8, 0.8, 1.0)
        .with_metallic(0.0)
        .with_roughness(0.5)
        .with_normal("textures/default_normal.png")
        .with_emissive(0.0, 0.0, 0.0)
        .with_render_state(RenderState {
            blend_mode: BlendMode::Opaque,
            cull_mode: CullMode::Back,
            depth_write: true,
            depth_test: true,
            wireframe: false,
        })
}

/// 2D sprite material with colour tint.
pub fn sprite_material() -> MaterialDefinition {
    MaterialDefinition::new("sprite", "sprite")
        .with_albedo_texture("textures/sprite.png")
        .with_albedo(1.0, 1.0, 1.0, 1.0)
        .with_render_state(RenderState {
            blend_mode: BlendMode::AlphaBlend,
            cull_mode: CullMode::None,
            depth_write: false,
            depth_test: false,
            wireframe: false,
        })
}

/// Additive-blend particle material.
pub fn particle_material() -> MaterialDefinition {
    MaterialDefinition::new("particle", "particle")
        .with_albedo_texture("textures/particle.png")
        .with_albedo(1.0, 1.0, 1.0, 1.0)
        .with_render_state(RenderState {
            blend_mode: BlendMode::Additive,
            cull_mode: CullMode::None,
            depth_write: false,
            depth_test: true,
            wireframe: false,
        })
}

/// UI material with alpha blending.
pub fn ui_material() -> MaterialDefinition {
    MaterialDefinition::new("ui", "ui")
        .with_albedo_texture("textures/ui_element.png")
        .with_albedo(1.0, 1.0, 1.0, 1.0)
        .with_opacity(1.0)
        .with_render_state(RenderState {
            blend_mode: BlendMode::AlphaBlend,
            cull_mode: CullMode::None,
            depth_write: false,
            depth_test: false,
            wireframe: false,
        })
}

/// Cubemap skybox material.
pub fn skybox_material() -> MaterialDefinition {
    let mut mat = MaterialDefinition::new("skybox", "skybox");
    mat.properties.push(MaterialProperty::new(
        "cubemap",
        MaterialValue::Texture("textures/skyboxcubemap.png".to_string()),
    ));
    mat.render_state = RenderState {
        blend_mode: BlendMode::Opaque,
        cull_mode: CullMode::None,
        depth_write: false,
        depth_test: false,
        wireframe: false,
    };
    mat
}

/// Multi-layer terrain material with a splatmap controlling blend weights.
pub fn terrain_material() -> MaterialDefinition {
    MaterialDefinition::new("terrain", "terrain")
        .with_albedo(0.6, 0.55, 0.4, 1.0)
        .with_albedo_texture("textures/grass.png")
        .with_property(MaterialProperty::new(
            "layer1",
            MaterialValue::Texture("textures/rock.png".to_string()),
        ))
        .with_property(MaterialProperty::new(
            "layer2",
            MaterialValue::Texture("textures/dirt.png".to_string()),
        ))
        .with_property(MaterialProperty::new(
            "splatmap",
            MaterialValue::Texture("textures/terrain_splat.png".to_string()),
        ))
        .with_normal("textures/terrain_normal.png")
        .with_roughness(0.9)
        .with_render_state(RenderState {
            blend_mode: BlendMode::Opaque,
            cull_mode: CullMode::Back,
            depth_write: true,
            depth_test: true,
            wireframe: false,
        })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 1. MaterialDefinition creation -----------------------------------
    #[test]
    fn test_material_creation() {
        let mat = MaterialDefinition::new("test", "unlit");
        assert_eq!(mat.name, "test");
        assert_eq!(mat.shader_name, "unlit");
        assert!(mat.properties.is_empty());
        assert_eq!(mat.render_state.blend_mode, BlendMode::Opaque);
        assert_eq!(mat.render_state.cull_mode, CullMode::Back);
        assert!(mat.render_state.depth_write);
        assert!(mat.render_state.depth_test);
        assert!(!mat.render_state.wireframe);
    }

    // --- 2. Property get / set --------------------------------------------
    #[test]
    fn test_property_get_set() {
        let mut mat = MaterialDefinition::new("mat", "pbr")
            .with_albedo(1.0, 0.0, 0.0, 1.0)
            .with_metallic(0.5);

        assert_eq!(
            mat.get_property("albedo"),
            Some(&MaterialValue::Color([1.0, 0.0, 0.0, 1.0]))
        );
        assert_eq!(
            mat.get_property("metallic"),
            Some(&MaterialValue::Float(0.5))
        );
        assert_eq!(mat.get_property("nonexistent"), None);

        mat.set_property("metallic", MaterialValue::Float(1.0))
            .unwrap();
        assert_eq!(
            mat.get_property("metallic"),
            Some(&MaterialValue::Float(1.0))
        );
    }

    // --- 3. Property type mismatch error ----------------------------------
    #[test]
    fn test_property_type_mismatch() {
        let mut mat = MaterialDefinition::new("mat", "pbr").with_metallic(0.5);

        let err = mat.set_property("metallic", MaterialValue::Int(1));
        assert!(matches!(err, Err(MaterialError::TypeMismatch(_))));
    }

    // --- 4. Multiple properties -------------------------------------------
    #[test]
    fn test_multiple_properties() {
        let mat = MaterialDefinition::new("multi", "pbr")
            .with_albedo(1.0, 1.0, 1.0, 1.0)
            .with_metallic(0.3)
            .with_roughness(0.7)
            .with_emissive(0.1, 0.0, 0.2);

        assert_eq!(mat.properties.len(), 4);
        assert!(mat.get_property("albedo").is_some());
        assert!(mat.get_property("metallic").is_some());
        assert!(mat.get_property("roughness").is_some());
        assert!(mat.get_property("emissive").is_some());
    }

    // --- 5. RenderState defaults ------------------------------------------
    #[test]
    fn test_render_state_defaults() {
        let rs = RenderState::default();
        assert_eq!(rs.blend_mode, BlendMode::Opaque);
        assert_eq!(rs.cull_mode, CullMode::Back);
        assert!(rs.depth_write);
        assert!(rs.depth_test);
        assert!(!rs.wireframe);
    }

    // --- 6a. Built-in: unlit ---------------------------------------------
    #[test]
    fn test_builtin_unlit() {
        let mat = unlit();
        assert_eq!(mat.name, "unlit");
        assert_eq!(mat.shader_name, "unlit");
        assert_eq!(mat.render_state.blend_mode, BlendMode::Opaque);
        assert!(mat.get_property("albedo").is_some());
    }

    // --- 6b. Built-in: pbr_standard --------------------------------------
    #[test]
    fn test_builtin_pbr() {
        let mat = pbr_standard();
        assert_eq!(mat.name, "pbr_standard");
        assert_eq!(mat.shader_name, "pbr");
        assert!(mat.get_property("metallic").is_some());
        assert!(mat.get_property("roughness").is_some());
        assert!(mat.get_property("normal_map").is_some());
    }

    // --- 6c. Built-in: sprite_material ------------------------------------
    #[test]
    fn test_builtin_sprite() {
        let mat = sprite_material();
        assert_eq!(mat.render_state.blend_mode, BlendMode::AlphaBlend);
        assert_eq!(mat.render_state.cull_mode, CullMode::None);
        assert!(mat.get_property("albedo_map").is_some());
    }

    // --- 6d. Built-in: particle_material ----------------------------------
    #[test]
    fn test_builtin_particle() {
        let mat = particle_material();
        assert_eq!(mat.render_state.blend_mode, BlendMode::Additive);
        assert!(!mat.render_state.depth_write);
    }

    // --- 6e. Built-in: ui_material ----------------------------------------
    #[test]
    fn test_builtin_ui() {
        let mat = ui_material();
        assert_eq!(mat.render_state.blend_mode, BlendMode::AlphaBlend);
        assert!(mat.get_property("opacity").is_some());
    }

    // --- 6f. Built-in: skybox_material ------------------------------------
    #[test]
    fn test_builtin_skybox() {
        let mat = skybox_material();
        assert_eq!(mat.render_state.cull_mode, CullMode::None);
        assert!(!mat.render_state.depth_write);
        let cubemap = mat.get_property("cubemap");
        assert!(matches!(cubemap, Some(MaterialValue::Texture(_))));
    }

    // --- 6g. Built-in: terrain_material -----------------------------------
    #[test]
    fn test_builtin_terrain() {
        let mat = terrain_material();
        assert_eq!(mat.name, "terrain");
        assert!(mat.get_property("splatmap").is_some());
        assert!(mat.get_property("layer1").is_some());
        assert!(mat.get_property("layer2").is_some());
    }

    // --- 7. MaterialValue variants ----------------------------------------
    #[test]
    fn test_material_value_variants() {
        let v = MaterialValue::Float(1.0);
        assert_eq!(v.uniform_size(), 4);
        assert_eq!(v.type_name(), "Float");

        let v = MaterialValue::Vec2([1.0, 2.0]);
        assert_eq!(v.uniform_size(), 8);

        let v = MaterialValue::Vec3([1.0, 2.0, 3.0]);
        assert_eq!(v.uniform_size(), 12);

        let v = MaterialValue::Vec4([1.0, 2.0, 3.0, 4.0]);
        assert_eq!(v.uniform_size(), 16);

        let v = MaterialValue::Color([1.0, 1.0, 1.0, 1.0]);
        assert_eq!(v.uniform_size(), 16);

        let v = MaterialValue::Texture("x.png".to_string());
        assert_eq!(v.uniform_size(), 0);

        let v = MaterialValue::Bool(true);
        assert_eq!(v.uniform_size(), 4);

        let v = MaterialValue::Int(42);
        assert_eq!(v.uniform_size(), 4);
    }

    // --- 8. Compile material ----------------------------------------------
    #[test]
    fn test_compile_material() {
        let mat = MaterialDefinition::new("compiled", "test_shader")
            .with_albedo(1.0, 0.5, 0.0, 1.0)
            .with_metallic(0.8)
            .with_albedo_texture("textures/a.png");

        let compiled = mat.compile();
        assert_eq!(compiled.shader_name, "test_shader");
        assert!(!compiled.uniform_data.is_empty());
        assert_eq!(
            compiled.uniform_data.len() % 16,
            0,
            "uniform data should be 16-byte aligned"
        );
        assert_eq!(compiled.texture_paths, vec!["textures/a.png"]);
        assert_eq!(compiled.render_state.blend_mode, BlendMode::Opaque);
    }

    // --- 9. BlendMode and CullMode variants -------------------------------
    #[test]
    fn test_blend_and_cull_variants() {
        let modes = [
            BlendMode::Opaque,
            BlendMode::AlphaBlend,
            BlendMode::Additive,
            BlendMode::Multiply,
        ];
        assert_eq!(modes.len(), 4);

        let culls = [CullMode::None, CullMode::Front, CullMode::Back];
        assert_eq!(culls.len(), 3);

        // verify they can be used in render state
        let rs = RenderState {
            blend_mode: BlendMode::Multiply,
            cull_mode: CullMode::Front,
            depth_write: false,
            depth_test: false,
            wireframe: true,
        };
        assert_eq!(rs.blend_mode, BlendMode::Multiply);
        assert_eq!(rs.cull_mode, CullMode::Front);
        assert!(rs.wireframe);
    }

    // --- 10. MaterialError display ----------------------------------------
    #[test]
    fn test_material_error_display() {
        let e = MaterialError::PropertyNotFound("albedo".to_string());
        assert!(e.to_string().contains("albedo"));

        let e = MaterialError::TypeMismatch("expected Float".to_string());
        assert!(e.to_string().contains("expected Float"));

        let e = MaterialError::CompilationFailed("bad shader".to_string());
        assert!(e.to_string().contains("bad shader"));
    }

    // --- 11. MaterialValue write_bytes ------------------------------------
    #[test]
    fn test_value_write_bytes() {
        let mut buf = Vec::new();
        MaterialValue::Float(1.0).write_bytes(&mut buf);
        assert_eq!(buf.len(), 4);

        buf.clear();
        MaterialValue::Vec3([1.0, 2.0, 3.0]).write_bytes(&mut buf);
        assert_eq!(buf.len(), 12);

        buf.clear();
        MaterialValue::Bool(true).write_bytes(&mut buf);
        assert_eq!(buf.len(), 4);
        let val = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(val, 1);

        buf.clear();
        MaterialValue::Int(-7).write_bytes(&mut buf);
        assert_eq!(buf.len(), 4);
        let val = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(val, -7);

        // Texture writes nothing to uniform buffer
        buf.clear();
        MaterialValue::Texture("x.png".to_string()).write_bytes(&mut buf);
        assert!(buf.is_empty());
    }

    // --- 12. Property not found on set ------------------------------------
    #[test]
    fn test_set_missing_property() {
        let mut mat = MaterialDefinition::new("m", "s");
        let result = mat.set_property("nope", MaterialValue::Float(1.0));
        assert!(matches!(result, Err(MaterialError::PropertyNotFound(_))));
    }

    // --- 13. Builder chaining with render state ---------------------------
    #[test]
    fn test_builder_render_state() {
        let mat = MaterialDefinition::new("chain", "shader")
            .with_albedo(1.0, 1.0, 1.0, 1.0)
            .with_blend_mode(BlendMode::Additive)
            .with_cull_mode(CullMode::None)
            .with_wireframe(true);

        assert_eq!(mat.render_state.blend_mode, BlendMode::Additive);
        assert_eq!(mat.render_state.cull_mode, CullMode::None);
        assert!(mat.render_state.wireframe);
    }

    // --- 14. Compile with no properties -----------------------------------
    #[test]
    fn test_compile_empty_material() {
        let mat = MaterialDefinition::new("empty", "fallback");
        let compiled = mat.compile();
        assert!(compiled.uniform_data.is_empty() || compiled.uniform_data.iter().all(|&b| b == 0));
        assert!(compiled.texture_paths.is_empty());
    }
}
