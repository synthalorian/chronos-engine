#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Shader graph data structures and WGSL code generation for Chronos Engine.
//!
//! Provides a node-based shader graph (data only — no visual editor), built-in
//! shader presets, and a simple polling-based hot-reload watcher.

use std::collections::HashMap;
use std::fmt;

// ---------------------------------------------------------------------------
// PortType
// ---------------------------------------------------------------------------

/// The data type carried by a shader port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,
    Texture,
    Bool,
}

impl PortType {
    /// The WGSL type name for this port type.
    pub fn wgsl_name(&self) -> &'static str {
        match self {
            PortType::Float => "f32",
            PortType::Vec2 => "vec2<f32>",
            PortType::Vec3 => "vec3<f32>",
            PortType::Vec4 => "vec4<f32>",
            PortType::Color => "vec4<f32>",
            PortType::Texture => "texture_2d<f32>",
            PortType::Bool => "bool",
        }
    }

    /// Whether two port types are compatible for connection.
    pub fn is_compatible_with(&self, other: &PortType) -> bool {
        // Colour and Vec4 are interchangeable; everything else must match exactly.
        if matches!(self, PortType::Color | PortType::Vec4)
            && matches!(other, PortType::Color | PortType::Vec4)
        {
            return true;
        }
        self == other
    }
}

// ---------------------------------------------------------------------------
// PortDef / NodePort / NodeConnection
// ---------------------------------------------------------------------------

/// Definition of a single input or output port on a node.
#[derive(Debug, Clone)]
pub struct PortDef {
    pub name: String,
    pub port_type: PortType,
}

impl PortDef {
    pub fn new(name: &str, port_type: PortType) -> Self {
        PortDef {
            name: name.to_string(),
            port_type,
        }
    }
}

/// A reference to a specific port on a specific node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodePort {
    pub node_id: usize,
    pub port_index: usize,
}

impl NodePort {
    pub fn new(node_id: usize, port_index: usize) -> Self {
        NodePort {
            node_id,
            port_index,
        }
    }
}

/// A directed connection from one node's output to another node's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeConnection {
    pub from: NodePort,
    pub to: NodePort,
}

impl NodeConnection {
    pub fn new(from: NodePort, to: NodePort) -> Self {
        NodeConnection { from, to }
    }
}

// ---------------------------------------------------------------------------
// ShaderNodeType
// ---------------------------------------------------------------------------

/// The kind of operation a shader node performs.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderNodeType {
    // -- Math -----------------------------------------------------------------
    Add,
    Subtract,
    Multiply,
    Divide,
    Sin,
    Cos,
    Sqrt,
    Abs,
    Clamp,
    Lerp,
    Step,
    Smoothstep,
    Pow,
    Min,
    Max,
    // -- Input ----------------------------------------------------------------
    Time,
    UV,
    WorldPosition,
    ViewDirection,
    Normal,
    ScreenPosition,
    // -- Texture --------------------------------------------------------------
    SampleTexture,
    SampleCubemap,
    // -- Output ---------------------------------------------------------------
    FragmentColor,
    WorldNormal,
    Emissive,
    Opacity,
    // -- Constants ------------------------------------------------------------
    FloatConstant(f32),
    ColorConstant([f32; 4]),
    Vec3Constant([f32; 3]),
}

impl ShaderNodeType {
    /// Human-readable name used in code generation and debugging.
    pub fn label(&self) -> String {
        match self {
            ShaderNodeType::Add => "add".into(),
            ShaderNodeType::Subtract => "subtract".into(),
            ShaderNodeType::Multiply => "multiply".into(),
            ShaderNodeType::Divide => "divide".into(),
            ShaderNodeType::Sin => "sin".into(),
            ShaderNodeType::Cos => "cos".into(),
            ShaderNodeType::Sqrt => "sqrt".into(),
            ShaderNodeType::Abs => "abs".into(),
            ShaderNodeType::Clamp => "clamp".into(),
            ShaderNodeType::Lerp => "lerp".into(),
            ShaderNodeType::Step => "step".into(),
            ShaderNodeType::Smoothstep => "smoothstep".into(),
            ShaderNodeType::Pow => "pow".into(),
            ShaderNodeType::Min => "min".into(),
            ShaderNodeType::Max => "max".into(),
            ShaderNodeType::Time => "time".into(),
            ShaderNodeType::UV => "uv".into(),
            ShaderNodeType::WorldPosition => "world_position".into(),
            ShaderNodeType::ViewDirection => "view_direction".into(),
            ShaderNodeType::Normal => "normal".into(),
            ShaderNodeType::ScreenPosition => "screen_position".into(),
            ShaderNodeType::SampleTexture => "sample_texture".into(),
            ShaderNodeType::SampleCubemap => "sample_cubemap".into(),
            ShaderNodeType::FragmentColor => "fragment_color".into(),
            ShaderNodeType::WorldNormal => "world_normal".into(),
            ShaderNodeType::Emissive => "emissive".into(),
            ShaderNodeType::Opacity => "opacity".into(),
            ShaderNodeType::FloatConstant(v) => format!("float_const({})", v),
            ShaderNodeType::ColorConstant(c) => format!("color_const({:?})", c),
            ShaderNodeType::Vec3Constant(v) => format!("vec3_const({:?})", v),
        }
    }

    /// Build the default input/output port definitions for this node type.
    pub fn default_ports(&self) -> (Vec<PortDef>, Vec<PortDef>) {
        match self {
            // Binary math
            ShaderNodeType::Add
            | ShaderNodeType::Subtract
            | ShaderNodeType::Multiply
            | ShaderNodeType::Divide
            | ShaderNodeType::Min
            | ShaderNodeType::Max => (
                vec![
                    PortDef::new("a", PortType::Float),
                    PortDef::new("b", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            // Unary math
            ShaderNodeType::Sin
            | ShaderNodeType::Cos
            | ShaderNodeType::Sqrt
            | ShaderNodeType::Abs => (
                vec![PortDef::new("in", PortType::Float)],
                vec![PortDef::new("out", PortType::Float)],
            ),
            ShaderNodeType::Clamp => (
                vec![
                    PortDef::new("value", PortType::Float),
                    PortDef::new("min", PortType::Float),
                    PortDef::new("max", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            ShaderNodeType::Lerp => (
                vec![
                    PortDef::new("a", PortType::Float),
                    PortDef::new("b", PortType::Float),
                    PortDef::new("t", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            ShaderNodeType::Step => (
                vec![
                    PortDef::new("edge", PortType::Float),
                    PortDef::new("x", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            ShaderNodeType::Smoothstep => (
                vec![
                    PortDef::new("edge0", PortType::Float),
                    PortDef::new("edge1", PortType::Float),
                    PortDef::new("x", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            ShaderNodeType::Pow => (
                vec![
                    PortDef::new("base", PortType::Float),
                    PortDef::new("exp", PortType::Float),
                ],
                vec![PortDef::new("out", PortType::Float)],
            ),
            // Inputs — no input ports
            ShaderNodeType::Time => (vec![], vec![PortDef::new("out", PortType::Float)]),
            ShaderNodeType::UV => (vec![], vec![PortDef::new("out", PortType::Vec2)]),
            ShaderNodeType::WorldPosition => (vec![], vec![PortDef::new("out", PortType::Vec3)]),
            ShaderNodeType::ViewDirection => (vec![], vec![PortDef::new("out", PortType::Vec3)]),
            ShaderNodeType::Normal => (vec![], vec![PortDef::new("out", PortType::Vec3)]),
            ShaderNodeType::ScreenPosition => (vec![], vec![PortDef::new("out", PortType::Vec4)]),
            // Texture
            ShaderNodeType::SampleTexture => (
                vec![
                    PortDef::new("uv", PortType::Vec2),
                    PortDef::new("texture", PortType::Texture),
                ],
                vec![PortDef::new("color", PortType::Vec4)],
            ),
            ShaderNodeType::SampleCubemap => (
                vec![
                    PortDef::new("dir", PortType::Vec3),
                    PortDef::new("texture", PortType::Texture),
                ],
                vec![PortDef::new("color", PortType::Vec4)],
            ),
            // Outputs
            ShaderNodeType::FragmentColor => (vec![PortDef::new("color", PortType::Vec4)], vec![]),
            ShaderNodeType::WorldNormal => (vec![PortDef::new("normal", PortType::Vec3)], vec![]),
            ShaderNodeType::Emissive => (vec![PortDef::new("color", PortType::Vec3)], vec![]),
            ShaderNodeType::Opacity => (vec![PortDef::new("value", PortType::Float)], vec![]),
            // Constants — no inputs
            ShaderNodeType::FloatConstant(_) => {
                (vec![], vec![PortDef::new("out", PortType::Float)])
            }
            ShaderNodeType::ColorConstant(_) => {
                (vec![], vec![PortDef::new("out", PortType::Color)])
            }
            ShaderNodeType::Vec3Constant(_) => (vec![], vec![PortDef::new("out", PortType::Vec3)]),
        }
    }

    /// The WGSL expression fragment for this node, given resolved input variable names.
    pub fn wgsl_expr(&self, input_vars: &[String]) -> String {
        match self {
            ShaderNodeType::Add => format!("({} + {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Subtract => format!("({} - {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Multiply => format!("({} * {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Divide => format!("({} / {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Sin => format!("sin({})", input_vars[0]),
            ShaderNodeType::Cos => format!("cos({})", input_vars[0]),
            ShaderNodeType::Sqrt => format!("sqrt({})", input_vars[0]),
            ShaderNodeType::Abs => format!("abs({})", input_vars[0]),
            ShaderNodeType::Clamp => format!(
                "clamp({}, {}, {})",
                input_vars[0], input_vars[1], input_vars[2]
            ),
            ShaderNodeType::Lerp => format!(
                "mix({}, {}, {})",
                input_vars[0], input_vars[1], input_vars[2]
            ),
            ShaderNodeType::Step => format!("step({}, {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Smoothstep => format!(
                "smoothstep({}, {}, {})",
                input_vars[0], input_vars[1], input_vars[2]
            ),
            ShaderNodeType::Pow => format!("pow({}, {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Min => format!("min({}, {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Max => format!("max({}, {})", input_vars[0], input_vars[1]),
            ShaderNodeType::Time => "uniforms.time".to_string(),
            ShaderNodeType::UV => "input_uv".to_string(),
            ShaderNodeType::WorldPosition => "input_world_pos".to_string(),
            ShaderNodeType::ViewDirection => "input_view_dir".to_string(),
            ShaderNodeType::Normal => "input_normal".to_string(),
            ShaderNodeType::ScreenPosition => "input_screen_pos".to_string(),
            ShaderNodeType::SampleTexture => format!(
                "textureSample({}, sampler, {})",
                input_vars[1], input_vars[0]
            ),
            ShaderNodeType::SampleCubemap => format!(
                "textureSample({}, sampler, {})",
                input_vars[1], input_vars[0]
            ),
            ShaderNodeType::FragmentColor => String::new(),
            ShaderNodeType::WorldNormal => String::new(),
            ShaderNodeType::Emissive => String::new(),
            ShaderNodeType::Opacity => String::new(),
            ShaderNodeType::FloatConstant(v) => format!("{}f", v),
            ShaderNodeType::ColorConstant(c) => {
                format!("vec4<f32>({}f, {}f, {}f, {}f)", c[0], c[1], c[2], c[3])
            }
            ShaderNodeType::Vec3Constant(v) => {
                format!("vec3<f32>({}f, {}f, {}f)", v[0], v[1], v[2])
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ShaderNode
// ---------------------------------------------------------------------------

/// A single node in the shader graph.
#[derive(Debug, Clone)]
pub struct ShaderNode {
    pub id: usize,
    pub node_type: ShaderNodeType,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub position: [f32; 2],
}

impl ShaderNode {
    /// Create a node with auto-assigned ports based on its type.
    pub fn new(id: usize, node_type: ShaderNodeType) -> Self {
        let (inputs, outputs) = node_type.default_ports();
        ShaderNode {
            id,
            node_type,
            inputs,
            outputs,
            position: [0.0, 0.0],
        }
    }

    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }
}

// ---------------------------------------------------------------------------
// ShaderInput / ShaderOutput
// ---------------------------------------------------------------------------

/// A named input to the overall shader graph (e.g. a uniform).
#[derive(Debug, Clone)]
pub struct ShaderInput {
    pub name: String,
    pub input_type: PortType,
    pub default_value: String,
}

impl ShaderInput {
    pub fn new(name: &str, input_type: PortType, default_value: &str) -> Self {
        ShaderInput {
            name: name.to_string(),
            input_type,
            default_value: default_value.to_string(),
        }
    }
}

/// A named output from the overall shader graph (e.g. fragment colour).
#[derive(Debug, Clone)]
pub struct ShaderOutput {
    pub name: String,
    pub output_type: PortType,
}

impl ShaderOutput {
    pub fn new(name: &str, output_type: PortType) -> Self {
        ShaderOutput {
            name: name.to_string(),
            output_type,
        }
    }
}

// ---------------------------------------------------------------------------
// ShaderError
// ---------------------------------------------------------------------------

/// Errors produced by shader graph operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderError {
    CycleDetected,
    UnconnectedInput { node: usize, port: String },
    TypeMismatch,
    InvalidConnection,
    GenerationFailed(String),
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderError::CycleDetected => write!(f, "shader graph contains a cycle"),
            ShaderError::UnconnectedInput { node, port } => {
                write!(f, "unconnected input on node {}: {}", node, port)
            }
            ShaderError::TypeMismatch => write!(f, "port type mismatch"),
            ShaderError::InvalidConnection => write!(f, "invalid connection"),
            ShaderError::GenerationFailed(msg) => write!(f, "shader generation failed: {}", msg),
        }
    }
}

impl std::error::Error for ShaderError {}

// ---------------------------------------------------------------------------
// ShaderGraph
// ---------------------------------------------------------------------------

/// A directed acyclic graph of shader nodes producing a shader program.
#[derive(Debug, Clone)]
pub struct ShaderGraph {
    pub name: String,
    pub nodes: Vec<ShaderNode>,
    pub connections: Vec<NodeConnection>,
    pub inputs: Vec<ShaderInput>,
    pub outputs: Vec<ShaderOutput>,
}

impl ShaderGraph {
    pub fn new(name: &str) -> Self {
        ShaderGraph {
            name: name.to_string(),
            nodes: Vec::new(),
            connections: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add a node and return its index.
    pub fn add_node(&mut self, node: ShaderNode) -> usize {
        let id = node.id;
        self.nodes.push(node);
        id
    }

    /// Connect an output port to an input port.
    pub fn connect(&mut self, from: NodePort, to: NodePort) -> Result<(), ShaderError> {
        // Validate that the source node and port exist.
        let from_node = self.nodes.iter().find(|n| n.id == from.node_id);
        let to_node = self.nodes.iter().find(|n| n.id == to.node_id);

        let from_n = match from_node {
            Some(n) => n,
            None => return Err(ShaderError::InvalidConnection),
        };
        let to_n = match to_node {
            Some(n) => n,
            None => return Err(ShaderError::InvalidConnection),
        };

        if from.port_index >= from_n.outputs.len() {
            return Err(ShaderError::InvalidConnection);
        }
        if to.port_index >= to_n.inputs.len() {
            return Err(ShaderError::InvalidConnection);
        }

        // Check type compatibility.
        let out_type = from_n.outputs[from.port_index].port_type;
        let in_type = to_n.inputs[to.port_index].port_type;
        if !out_type.is_compatible_with(&in_type) {
            return Err(ShaderError::TypeMismatch);
        }

        self.connections.push(NodeConnection::new(from, to));
        Ok(())
    }

    /// Check the graph for structural problems: cycles and unconnected inputs.
    pub fn validate(&self) -> Result<(), ShaderError> {
        // Check for cycles via DFS.
        if self.has_cycle() {
            return Err(ShaderError::CycleDetected);
        }

        // Check for unconnected inputs (only on non-output nodes).
        for node in &self.nodes {
            // Output-type nodes (FragmentColor, etc.) are expected to have
            // unconnected *outputs* but must have their inputs connected.
            for (i, port) in node.inputs.iter().enumerate() {
                let connected = self
                    .connections
                    .iter()
                    .any(|c| c.to.node_id == node.id && c.to.port_index == i);
                if !connected {
                    return Err(ShaderError::UnconnectedInput {
                        node: node.id,
                        port: port.name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Topological-sort the nodes for code generation.
    fn topological_order(&self) -> Vec<usize> {
        let n = self.nodes.len();
        if n == 0 {
            return Vec::new();
        }

        // Build adjacency list (from -> list of to node ids).
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut in_degree: HashMap<usize, usize> = HashMap::new();

        for node in &self.nodes {
            adj.entry(node.id).or_default();
            in_degree.entry(node.id).or_insert(0);
        }

        for conn in &self.connections {
            adj.entry(conn.from.node_id)
                .or_default()
                .push(conn.to.node_id);
            *in_degree.entry(conn.to.node_id).or_insert(0) += 1;
        }

        let mut queue: Vec<usize> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order = Vec::new();
        while let Some(id) = queue.pop() {
            order.push(id);
            if let Some(neighbours) = adj.get(&id) {
                for &nbr in neighbours {
                    if let Some(deg) = in_degree.get_mut(&nbr) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(nbr);
                        }
                    }
                }
            }
        }

        order
    }

    /// Detect whether the graph contains any cycle.
    fn has_cycle(&self) -> bool {
        let order = self.topological_order();
        order.len() < self.nodes.len()
    }

    /// Generate a WGSL shader string from this graph.
    pub fn generate_wgsl(&self) -> Result<String, ShaderError> {
        // Allow generation even on incomplete graphs — just produce whatever we can.
        let order = self.topological_order();
        let mut lines: Vec<String> = Vec::new();

        lines.push("// Auto-generated shader".into());
        lines.push(format!("// Graph: {}", self.name));
        lines.push(String::new());

        // Struct declarations
        lines.push("struct Uniforms {".into());
        for input in &self.inputs {
            lines.push(format!(
                "    {}: {},",
                input.name,
                input.input_type.wgsl_name()
            ));
        }
        lines.push("};".into());
        lines.push(String::new());

        lines.push("@group(0) @binding(0) var<uniform> uniforms: Uniforms;".into());
        lines.push(String::new());

        lines.push("struct VSOutput {".into());
        lines.push("    @builtin(position) pos: vec4<f32>,".into());
        lines.push("    @location(0) uv: vec2<f32>,".into());
        lines.push("    @location(1) normal: vec3<f32>,".into());
        lines.push("    @location(2) world_pos: vec3<f32>,".into());
        lines.push("};".into());
        lines.push(String::new());

        // Emit node variable declarations in topological order.
        let mut var_names: HashMap<(usize, usize), String> = HashMap::new();

        for node_id in &order {
            let node = match self.nodes.iter().find(|n| n.id == *node_id) {
                Some(n) => n,
                None => continue,
            };

            // Resolve input variable names.
            let mut input_vars: Vec<String> = Vec::new();
            for i in 0..node.inputs.len() {
                // Find the connection feeding this input.
                let conn = self
                    .connections
                    .iter()
                    .find(|c| c.to.node_id == node.id && c.to.port_index == i);
                if let Some(c) = conn {
                    if let Some(name) = var_names.get(&(c.from.node_id, c.from.port_index)) {
                        input_vars.push(name.clone());
                    } else {
                        input_vars.push(format!("/* unresolvable input {} */", i));
                    }
                } else {
                    input_vars.push(format!("/* unconnected input {} */", i));
                }
            }

            // Output nodes just assign to outputs; they produce no variable.
            match &node.node_type {
                ShaderNodeType::FragmentColor
                | ShaderNodeType::WorldNormal
                | ShaderNodeType::Emissive
                | ShaderNodeType::Opacity => {
                    // handled below in fragment output
                    for (oi, _) in node.outputs.iter().enumerate() {
                        var_names.insert((node.id, oi), format!("node_{}_out_{}", node.id, oi));
                    }
                    continue;
                }
                _ => {}
            }

            let expr = node.node_type.wgsl_expr(&input_vars);
            let var_name = format!("n{}", node.id);
            let out_type = if node.outputs.is_empty() {
                "f32".to_string()
            } else {
                node.outputs[0].port_type.wgsl_name().to_string()
            };

            lines.push(format!(
                "let {} = {}: {} = {};",
                var_name, var_name, out_type, expr
            ));

            for (oi, _) in node.outputs.iter().enumerate() {
                var_names.insert((node.id, oi), var_name.clone());
            }
        }

        lines.push(String::new());
        lines.push("// Fragment shader body".into());

        Ok(lines.join("\n"))
    }
}

// ---------------------------------------------------------------------------
// Built-in shader presets
// ---------------------------------------------------------------------------

/// Built-in unlit shader — outputs a flat colour.
pub fn unlit_shader() -> ShaderGraph {
    let mut graph = ShaderGraph::new("unlit");
    let color_node = ShaderNode::new(0, ShaderNodeType::ColorConstant([1.0, 1.0, 1.0, 1.0]));
    let output_node = ShaderNode::new(1, ShaderNodeType::FragmentColor);

    graph.add_node(color_node);
    graph.add_node(output_node);
    graph
        .connect(NodePort::new(0, 0), NodePort::new(1, 0))
        .expect("unlit shader: valid connection between color constant and fragment color");

    graph
        .outputs
        .push(ShaderOutput::new("color", PortType::Vec4));
    graph
}

/// Built-in PBR shader graph with texture sampling.
pub fn pbr_shader() -> ShaderGraph {
    let mut graph = ShaderGraph::new("pbr");

    let uv_node = ShaderNode::new(0, ShaderNodeType::UV);
    let tex_node = ShaderNode::new(1, ShaderNodeType::SampleTexture);
    let output_node = ShaderNode::new(2, ShaderNodeType::FragmentColor);

    graph.add_node(uv_node);
    graph.add_node(tex_node);
    graph.add_node(output_node);

    // UV -> texture.uv
    graph
        .connect(NodePort::new(0, 0), NodePort::new(1, 0))
        .expect("pbr shader: valid connection between UV and texture");
    // texture.color -> FragmentColor.color
    graph
        .connect(NodePort::new(1, 0), NodePort::new(2, 0))
        .expect("pbr shader: valid connection between texture and fragment color");

    graph.inputs.push(ShaderInput::new(
        "albedo",
        PortType::Color,
        "vec4<f32>(1.0, 1.0, 1.0, 1.0)",
    ));
    graph
        .outputs
        .push(ShaderOutput::new("color", PortType::Vec4));
    graph
}

/// Built-in sprite shader — samples a texture with UV.
pub fn sprite_shader() -> ShaderGraph {
    let mut graph = ShaderGraph::new("sprite");

    let uv_node = ShaderNode::new(0, ShaderNodeType::UV);
    let tex_node = ShaderNode::new(1, ShaderNodeType::SampleTexture);
    let output_node = ShaderNode::new(2, ShaderNodeType::FragmentColor);

    graph.add_node(uv_node);
    graph.add_node(tex_node);
    graph.add_node(output_node);

    graph
        .connect(NodePort::new(0, 0), NodePort::new(1, 0))
        .expect("sprite shader: valid connection between UV and texture");
    graph
        .connect(NodePort::new(1, 0), NodePort::new(2, 0))
        .expect("sprite shader: valid connection between texture and fragment color");

    graph
        .outputs
        .push(ShaderOutput::new("color", PortType::Vec4));
    graph
}

// ---------------------------------------------------------------------------
// ShaderWatcher (simple polling-based hot-reload)
// ---------------------------------------------------------------------------

/// Watches a set of file paths and reports which have been modified since the
/// last check. Uses a simple timestamp-based polling strategy with no external
/// dependencies.
#[derive(Debug, Clone)]
pub struct ShaderWatcher {
    watched_paths: Vec<String>,
    last_modified: HashMap<String, u64>,
}

impl ShaderWatcher {
    pub fn new() -> Self {
        ShaderWatcher {
            watched_paths: Vec::new(),
            last_modified: HashMap::new(),
        }
    }

    /// Begin watching a path.
    pub fn watch(&mut self, path: &str) {
        if !self.watched_paths.contains(&path.to_string()) {
            self.watched_paths.push(path.to_string());
            // Record the current modification time (0 = not yet checked).
            self.last_modified.insert(path.to_string(), 0);
        }
    }

    /// Return the list of watched paths whose modification timestamp is newer
    /// than what was recorded on the previous call. Updates the stored
    /// timestamps so the same change is not reported twice.
    pub fn check_changes(&mut self) -> Vec<String> {
        let mut changed = Vec::new();
        for path in &self.watched_paths.clone() {
            let current = Self::read_timestamp(path);
            let previous = self.last_modified.get(path).copied().unwrap_or(0);
            if previous > 0 && current > previous {
                changed.push(path.clone());
            }
            if current > 0 {
                self.last_modified.insert(path.clone(), current);
            }
        }
        changed
    }

    /// Read a simple monotonic timestamp for a file. Uses
    /// `std::fs::metadata` on real files; returns 0 for non-existent paths.
    fn read_timestamp(path: &str) -> u64 {
        std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Number of currently watched paths.
    pub fn watched_count(&self) -> usize {
        self.watched_paths.len()
    }
}

impl Default for ShaderWatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- 1. ShaderGraph creation ------------------------------------------
    #[test]
    fn test_shader_graph_creation() {
        let g = ShaderGraph::new("test");
        assert_eq!(g.name, "test");
        assert!(g.nodes.is_empty());
        assert!(g.connections.is_empty());
    }

    // --- 2. Add nodes -----------------------------------------------------
    #[test]
    fn test_add_nodes() {
        let mut g = ShaderGraph::new("g");
        let id0 = g.add_node(ShaderNode::new(0, ShaderNodeType::FloatConstant(1.0)));
        let id1 = g.add_node(ShaderNode::new(1, ShaderNodeType::Sin));
        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
        assert_eq!(g.nodes.len(), 2);
    }

    // --- 3. Connect ports -------------------------------------------------
    #[test]
    fn test_connect_ports() {
        let mut g = ShaderGraph::new("g");
        g.add_node(ShaderNode::new(0, ShaderNodeType::FloatConstant(1.0)));
        g.add_node(ShaderNode::new(1, ShaderNodeType::Sin));
        let result = g.connect(NodePort::new(0, 0), NodePort::new(1, 0));
        assert!(result.is_ok());
        assert_eq!(g.connections.len(), 1);
    }

    // --- 4. Validate — missing input error --------------------------------
    #[test]
    fn test_validate_missing_input() {
        let mut g = ShaderGraph::new("g");
        // Sin node needs an input but nothing is connected.
        g.add_node(ShaderNode::new(0, ShaderNodeType::Sin));
        let result = g.validate();
        assert!(matches!(
            result,
            Err(ShaderError::UnconnectedInput { node: 0, .. })
        ));
    }

    // --- 5. Validate — cycle detection ------------------------------------
    #[test]
    fn test_validate_cycle() {
        let mut g = ShaderGraph::new("g");
        // Create two nodes and connect them in a cycle:
        // We need nodes with matching input/output types.
        // Use Add (two Float inputs, one Float output).
        g.add_node(ShaderNode::new(0, ShaderNodeType::Add));
        g.add_node(ShaderNode::new(1, ShaderNodeType::Add));
        // 0.out -> 1.a  and  1.out -> 0.a  creates a cycle.
        g.connect(NodePort::new(0, 0), NodePort::new(1, 0)).unwrap();
        g.connect(NodePort::new(1, 0), NodePort::new(0, 0)).unwrap();
        assert!(matches!(g.validate(), Err(ShaderError::CycleDetected)));
    }

    // --- 6. NodeConnection creation ---------------------------------------
    #[test]
    fn test_node_connection() {
        let c = NodeConnection::new(NodePort::new(0, 0), NodePort::new(1, 0));
        assert_eq!(c.from.node_id, 0);
        assert_eq!(c.from.port_index, 0);
        assert_eq!(c.to.node_id, 1);
        assert_eq!(c.to.port_index, 0);
    }

    // --- 7. ShaderNodeType variants ---------------------------------------
    #[test]
    fn test_node_type_variants() {
        let types = vec![
            ShaderNodeType::Add,
            ShaderNodeType::Subtract,
            ShaderNodeType::Multiply,
            ShaderNodeType::Divide,
            ShaderNodeType::Sin,
            ShaderNodeType::Cos,
            ShaderNodeType::Sqrt,
            ShaderNodeType::Abs,
            ShaderNodeType::Clamp,
            ShaderNodeType::Lerp,
            ShaderNodeType::Step,
            ShaderNodeType::Smoothstep,
            ShaderNodeType::Pow,
            ShaderNodeType::Min,
            ShaderNodeType::Max,
            ShaderNodeType::Time,
            ShaderNodeType::UV,
            ShaderNodeType::WorldPosition,
            ShaderNodeType::ViewDirection,
            ShaderNodeType::Normal,
            ShaderNodeType::ScreenPosition,
            ShaderNodeType::SampleTexture,
            ShaderNodeType::SampleCubemap,
            ShaderNodeType::FragmentColor,
            ShaderNodeType::WorldNormal,
            ShaderNodeType::Emissive,
            ShaderNodeType::Opacity,
            ShaderNodeType::FloatConstant(1.0),
            ShaderNodeType::ColorConstant([1.0; 4]),
            ShaderNodeType::Vec3Constant([1.0; 3]),
        ];
        assert!(types.len() >= 20, "expected at least 20 node type variants");

        // Every variant should produce a non-empty label and ports.
        for t in &types {
            assert!(!t.label().is_empty());
            let (inputs, outputs) = t.default_ports();
            assert!(inputs.len() + outputs.len() > 0);
        }
    }

    // --- 8. PortType compatibility ----------------------------------------
    #[test]
    fn test_port_type_compatibility() {
        assert!(PortType::Float.is_compatible_with(&PortType::Float));
        assert!(!PortType::Float.is_compatible_with(&PortType::Vec3));
        assert!(PortType::Color.is_compatible_with(&PortType::Vec4));
        assert!(PortType::Vec4.is_compatible_with(&PortType::Color));
        assert!(PortType::Vec3.is_compatible_with(&PortType::Vec3));
        assert!(!PortType::Vec2.is_compatible_with(&PortType::Vec3));
    }

    // --- 9a. Built-in: unlit_shader ---------------------------------------
    #[test]
    fn test_builtin_unlit_shader() {
        let g = unlit_shader();
        assert_eq!(g.name, "unlit");
        assert_eq!(g.nodes.len(), 2);
        assert_eq!(g.connections.len(), 1);
    }

    // --- 9b. Built-in: pbr_shader -----------------------------------------
    #[test]
    fn test_builtin_pbr_shader() {
        let g = pbr_shader();
        assert_eq!(g.name, "pbr");
        assert_eq!(g.nodes.len(), 3);
        assert!(g.inputs.iter().any(|i| i.name == "albedo"));
    }

    // --- 9c. Built-in: sprite_shader --------------------------------------
    #[test]
    fn test_builtin_sprite_shader() {
        let g = sprite_shader();
        assert_eq!(g.name, "sprite");
        assert_eq!(g.nodes.len(), 3);
    }

    // --- 10. ShaderWatcher watch and check_changes ------------------------
    #[test]
    fn test_shader_watcher() {
        let mut watcher = ShaderWatcher::new();
        assert_eq!(watcher.watched_count(), 0);

        // Watch a path that doesn't exist on disk.
        watcher.watch("/tmp/chronos_test_does_not_exist.wgsl");
        assert_eq!(watcher.watched_count(), 1);

        // No changes expected (file doesn't exist or hasn't changed).
        let changes = watcher.check_changes();
        assert!(changes.is_empty());
    }

    #[test]
    fn test_shader_watcher_duplicate_watch() {
        let mut watcher = ShaderWatcher::new();
        watcher.watch("/tmp/test.wgsl");
        watcher.watch("/tmp/test.wgsl");
        assert_eq!(watcher.watched_count(), 1);
    }

    // --- 11. ShaderError display ------------------------------------------
    #[test]
    fn test_shader_error_display() {
        let e = ShaderError::CycleDetected;
        assert!(e.to_string().contains("cycle"));

        let e = ShaderError::UnconnectedInput {
            node: 3,
            port: "a".into(),
        };
        assert!(e.to_string().contains("node 3"));
        assert!(e.to_string().contains("a"));

        let e = ShaderError::TypeMismatch;
        assert!(e.to_string().contains("mismatch"));

        let e = ShaderError::InvalidConnection;
        assert!(e.to_string().contains("invalid"));

        let e = ShaderError::GenerationFailed("boom".into());
        assert!(e.to_string().contains("boom"));
    }

    // --- 12. generate_wgsl produces non-empty string ----------------------
    #[test]
    fn test_generate_wgsl() {
        let g = unlit_shader();
        let wgsl = g.generate_wgsl().unwrap();
        assert!(!wgsl.is_empty());
        assert!(wgsl.contains("Auto-generated"));
        assert!(wgsl.contains("unlit"));
    }

    // --- 13. Type mismatch on connect -------------------------------------
    #[test]
    fn test_connect_type_mismatch() {
        let mut g = ShaderGraph::new("g");
        g.add_node(ShaderNode::new(0, ShaderNodeType::UV)); // outputs Vec2
        g.add_node(ShaderNode::new(1, ShaderNodeType::Sin)); // expects Float input
        let result = g.connect(NodePort::new(0, 0), NodePort::new(1, 0));
        assert!(matches!(result, Err(ShaderError::TypeMismatch)));
    }

    // --- 14. Invalid connection — bad node id -----------------------------
    #[test]
    fn test_connect_invalid_node() {
        let mut g = ShaderGraph::new("g");
        g.add_node(ShaderNode::new(0, ShaderNodeType::FloatConstant(1.0)));
        // Node 99 doesn't exist.
        let result = g.connect(NodePort::new(0, 0), NodePort::new(99, 0));
        assert!(matches!(result, Err(ShaderError::InvalidConnection)));
    }

    // --- 15. PortType wgsl_name -------------------------------------------
    #[test]
    fn test_port_type_wgsl() {
        assert_eq!(PortType::Float.wgsl_name(), "f32");
        assert_eq!(PortType::Vec2.wgsl_name(), "vec2<f32>");
        assert_eq!(PortType::Vec3.wgsl_name(), "vec3<f32>");
        assert_eq!(PortType::Vec4.wgsl_name(), "vec4<f32>");
        assert_eq!(PortType::Color.wgsl_name(), "vec4<f32>");
        assert_eq!(PortType::Texture.wgsl_name(), "texture_2d<f32>");
        assert_eq!(PortType::Bool.wgsl_name(), "bool");
    }

    // --- 16. ShaderNode with_position -------------------------------------
    #[test]
    fn test_node_position() {
        let n = ShaderNode::new(0, ShaderNodeType::Add).with_position(100.0, 200.0);
        assert_eq!(n.position, [100.0, 200.0]);
    }

    // --- 17. Validate a fully-connected graph passes ----------------------
    #[test]
    fn test_validate_connected_graph() {
        let g = unlit_shader();
        assert!(g.validate().is_ok());
    }

    // --- 18. WGSL generation for PBR shader ------------------------------
    #[test]
    fn test_pbr_generate_wgsl() {
        let g = pbr_shader();
        let wgsl = g.generate_wgsl().unwrap();
        assert!(!wgsl.is_empty());
        assert!(wgsl.contains("pbr"));
    }
}
