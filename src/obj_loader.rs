//! Wavefront .obj file parser for loading 3D meshes.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ObjMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub faces: Vec<ObjFace>,
}

#[derive(Debug, Clone)]
pub struct ObjFace {
    pub vertices: Vec<ObjVertex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjVertex {
    pub position_index: u32,
    pub uv_index: Option<u32>,
    pub normal_index: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OBJ parse error at line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

impl ObjMesh {
    pub fn new() -> Self {
        ObjMesh {
            positions: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            faces: Vec::new(),
        }
    }

    pub fn parse(input: &str) -> Result<Self, ParseError> {
        let mut mesh = ObjMesh::new();

        for (line_num, raw_line) in input.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() < 4 {
                        return Err(ParseError {
                            line: line_num + 1,
                            message: "vertex requires at least 3 coordinates".into(),
                        });
                    }
                    let x = parse_f32(parts[1], line_num + 1)?;
                    let y = parse_f32(parts[2], line_num + 1)?;
                    let z = parse_f32(parts[3], line_num + 1)?;
                    mesh.positions.push([x, y, z]);
                }
                "vn" => {
                    if parts.len() < 4 {
                        return Err(ParseError {
                            line: line_num + 1,
                            message: "normal requires 3 components".into(),
                        });
                    }
                    let x = parse_f32(parts[1], line_num + 1)?;
                    let y = parse_f32(parts[2], line_num + 1)?;
                    let z = parse_f32(parts[3], line_num + 1)?;
                    mesh.normals.push([x, y, z]);
                }
                "vt" => {
                    if parts.len() < 3 {
                        return Err(ParseError {
                            line: line_num + 1,
                            message: "UV requires at least 2 components".into(),
                        });
                    }
                    let u = parse_f32(parts[1], line_num + 1)?;
                    let v = parse_f32(parts[2], line_num + 1)?;
                    mesh.uvs.push([u, v]);
                }
                "f" => {
                    if parts.len() < 4 {
                        return Err(ParseError {
                            line: line_num + 1,
                            message: "face requires at least 3 vertices".into(),
                        });
                    }
                    let mut vertices = Vec::new();
                    for &part in &parts[1..] {
                        vertices.push(parse_face_vertex(part, line_num + 1)?);
                    }
                    mesh.faces.push(ObjFace { vertices });
                }
                _ => {}
            }
        }

        Ok(mesh)
    }

    pub fn from_file(path: &str) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn triangulate(&self) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let mut index_map: HashMap<ObjVertex, u32> = HashMap::new();

        for face in &self.faces {
            if face.vertices.len() < 3 {
                continue;
            }

            let fan_base = get_or_insert_vertex(
                &face.vertices[0], &self, &mut index_map,
                &mut positions, &mut normals, &mut uvs,
            );

            for i in 2..face.vertices.len() {
                let v1 = get_or_insert_vertex(
                    &face.vertices[i - 1], &self, &mut index_map,
                    &mut positions, &mut normals, &mut uvs,
                );
                let v2 = get_or_insert_vertex(
                    &face.vertices[i], &self, &mut index_map,
                    &mut positions, &mut normals, &mut uvs,
                );
                indices.push(fan_base);
                indices.push(v1);
                indices.push(v2);
            }
        }

        (positions, normals, uvs, indices)
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.faces.iter().map(|f| (f.vertices.len() - 2).max(0)).sum::<usize>()
    }
}

fn get_or_insert_vertex(
    vert: &ObjVertex,
    mesh: &ObjMesh,
    index_map: &mut HashMap<ObjVertex, u32>,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
) -> u32 {
    if let Some(&idx) = index_map.get(vert) {
        return idx;
    }

    let idx = positions.len() as u32;
    let pi = vert.position_index as usize;
    if pi > 0 {
        positions.push(mesh.positions.get(pi - 1).copied().unwrap_or([0.0, 0.0, 0.0]));
    } else {
        positions.push([0.0, 0.0, 0.0]);
    }

    if let Some(ni) = vert.normal_index {
        normals.push(mesh.normals.get(ni as usize - 1).copied().unwrap_or([0.0, 1.0, 0.0]));
    } else {
        normals.push([0.0, 1.0, 0.0]);
    }

    if let Some(ui) = vert.uv_index {
        uvs.push(mesh.uvs.get(ui as usize - 1).copied().unwrap_or([0.0, 0.0]));
    } else {
        uvs.push([0.0, 0.0]);
    }

    index_map.insert(*vert, idx);
    idx
}

fn parse_f32(s: &str, line: usize) -> Result<f32, ParseError> {
    s.parse::<f32>().map_err(|_| ParseError {
        line,
        message: format!("invalid float: '{}'", s),
    })
}

fn parse_face_vertex(s: &str, line: usize) -> Result<ObjVertex, ParseError> {
    let parts: Vec<&str> = s.split('/').collect();

    let position_index = parts[0].parse::<u32>().map_err(|_| ParseError {
        line,
        message: format!("invalid vertex index: '{}'", parts[0]),
    })?;

    let uv_index = if parts.len() > 1 && !parts[1].is_empty() {
        Some(parts[1].parse::<u32>().map_err(|_| ParseError {
            line,
            message: format!("invalid UV index: '{}'", parts[1]),
        })?)
    } else {
        None
    };

    let normal_index = if parts.len() > 2 && !parts[2].is_empty() {
        Some(parts[2].parse::<u32>().map_err(|_| ParseError {
            line,
            message: format!("invalid normal index: '{}'", parts[2]),
        })?)
    } else {
        None
    };

    Ok(ObjVertex {
        position_index,
        uv_index,
        normal_index,
    })
}
