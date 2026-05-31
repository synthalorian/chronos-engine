#[cfg(feature = "game")]
use crate::input::{InputManager, InputSource, KeyCode};

// ──────────────────────────────────────────────
// Tabletop / Isometric Camera Controller
// ──────────────────────────────────────────────

/// RTS tabletop camera that orbits a target point with pan, zoom, and follow.
#[derive(Debug, Clone)]
pub struct TabletopCamera {
    pub target: [f32; 3],
    pub distance: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
    pub fov: f32,
    pub follow_target: Option<[f32; 3]>,
    pub follow_smoothing: f32,
}

impl TabletopCamera {
    /// Default isometric-ish view: pitch ~50 degrees, distance 20 units.
    pub fn new() -> Self {
        TabletopCamera {
            target: [0.0, 0.0, 0.0],
            distance: 20.0,
            pitch: 0.85,
            yaw: 0.0,
            pan_speed: 15.0,
            zoom_speed: 2.0,
            min_distance: 5.0,
            max_distance: 100.0,
            fov: std::f32::consts::FRAC_PI_4, // 45 degrees
            follow_target: None,
            follow_smoothing: 5.0,
        }
    }

    /// Process input from the engine's InputManager.
    ///
    /// Checks WASD for panning (relative to yaw), scroll for zoom,
    /// and Q/E for rotation.
    pub fn handle_input(&mut self, dt: f32, input: &InputManager) {
        let pan = self.pan_speed * dt;

        // Forward/backward relative to yaw direction (on XZ plane)
        let fwd_x = self.yaw.sin();
        let fwd_z = self.yaw.cos();
        let right_x = (self.yaw + std::f32::consts::FRAC_PI_2).sin();
        let right_z = (self.yaw + std::f32::consts::FRAC_PI_2).cos();

        if input.is_source_pressed(&InputSource::Key(KeyCode::W)) {
            self.target[0] += fwd_x * pan;
            self.target[2] += fwd_z * pan;
        }
        if input.is_source_pressed(&InputSource::Key(KeyCode::S)) {
            self.target[0] -= fwd_x * pan;
            self.target[2] -= fwd_z * pan;
        }
        if input.is_source_pressed(&InputSource::Key(KeyCode::A)) {
            self.target[0] -= right_x * pan;
            self.target[2] -= right_z * pan;
        }
        if input.is_source_pressed(&InputSource::Key(KeyCode::D)) {
            self.target[0] += right_x * pan;
            self.target[2] += right_z * pan;
        }

        // Scroll zoom
        let (_, scroll_y) = input.scroll_delta();
        if scroll_y != 0.0 {
            self.zoom(-scroll_y * self.zoom_speed);
        }

        // Rotation: Q = counter-clockwise, E = clockwise
        let rot_speed = 2.0 * dt;
        if input.is_source_pressed(&InputSource::Key(KeyCode::Q)) {
            self.rotate(rot_speed);
        }
        if input.is_source_pressed(&InputSource::Key(KeyCode::E)) {
            self.rotate(-rot_speed);
        }
    }

    /// Handle keyboard panning from a raw slice of pressed keys.
    pub fn handle_keyboard(&mut self, keys: &[KeyCode], dt: f32) {
        let pan = self.pan_speed * dt;
        let fwd_x = self.yaw.sin();
        let fwd_z = self.yaw.cos();
        let right_x = (self.yaw + std::f32::consts::FRAC_PI_2).sin();
        let right_z = (self.yaw + std::f32::consts::FRAC_PI_2).cos();

        for key in keys {
            match key {
                KeyCode::W => {
                    self.target[0] += fwd_x * pan;
                    self.target[2] += fwd_z * pan;
                }
                KeyCode::S => {
                    self.target[0] -= fwd_x * pan;
                    self.target[2] -= fwd_z * pan;
                }
                KeyCode::A => {
                    self.target[0] -= right_x * pan;
                    self.target[2] -= right_z * pan;
                }
                KeyCode::D => {
                    self.target[0] += right_x * pan;
                    self.target[2] += right_z * pan;
                }
                KeyCode::Q => self.rotate(2.0 * dt),
                KeyCode::E => self.rotate(-2.0 * dt),
                _ => {}
            }
        }
    }

    /// Handle scroll-based zoom.
    pub fn handle_scroll(&mut self, scroll_y: f32) {
        if scroll_y != 0.0 {
            self.zoom(-scroll_y * self.zoom_speed);
        }
    }

    /// Per-frame update — applies follow target smoothing.
    pub fn update(&mut self, dt: f32) {
        if let Some(follow) = self.follow_target {
            let t = (self.follow_smoothing * dt).min(1.0);
            self.target[0] += (follow[0] - self.target[0]) * t;
            self.target[1] += (follow[1] - self.target[1]) * t;
            self.target[2] += (follow[2] - self.target[2]) * t;
        }
    }

    /// Camera position from spherical coordinates around target.
    pub fn eye_position(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        [
            self.target[0] + self.distance * cp * sy,
            self.target[1] + self.distance * sp,
            self.target[2] + self.distance * cp * cy,
        ]
    }

    /// View matrix (look-at from eye to target with up = `[0, 1, 0]`).
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        let eye = self.eye_position();
        look_at(eye, self.target, [0.0, 1.0, 0.0])
    }

    /// Perspective projection matrix.
    pub fn projection_matrix(&self, aspect: f32) -> [[f32; 4]; 4] {
        perspective(self.fov, aspect, 0.1, 1000.0)
    }

    /// Unproject screen coordinates to a world-space ray for mouse picking.
    ///
    /// Returns (ray_origin, ray_direction).
    pub fn screen_to_ray(
        &self,
        screen_x: f32,
        screen_y: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> ([f32; 3], [f32; 3]) {
        let ndc_x = (2.0 * screen_x / screen_width) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / screen_height);

        let view = self.view_matrix();
        let proj = self.projection_matrix(screen_width / screen_height);
        let inv_vp = mat4_mul_mat4(invert_mat4(proj), invert_mat4(view));

        // Near and far points in world space
        let near = mat4_mul_vec4(inv_vp, [ndc_x, ndc_y, -1.0, 1.0]);
        let far = mat4_mul_vec4(inv_vp, [ndc_x, ndc_y, 1.0, 1.0]);

        // Perspective divide
        let near_w = if near[3].abs() < 1e-6 { 1.0 } else { near[3] };
        let far_w = if far[3].abs() < 1e-6 { 1.0 } else { far[3] };
        let near_pt = [near[0] / near_w, near[1] / near_w, near[2] / near_w];
        let far_pt = [far[0] / far_w, far[1] / far_w, far[2] / far_w];

        let dir = normalize3([
            far_pt[0] - near_pt[0],
            far_pt[1] - near_pt[1],
            far_pt[2] - near_pt[2],
        ]);

        (near_pt, dir)
    }

    /// Pan the camera target by world-space offsets.
    pub fn pan(&mut self, dx: f32, dz: f32, dt: f32) {
        self.target[0] += dx * dt;
        self.target[2] += dz * dt;
    }

    /// Zoom in/out, clamped to min/max distance.
    pub fn zoom(&mut self, amount: f32) {
        self.distance = (self.distance + amount).clamp(self.min_distance, self.max_distance);
    }

    /// Rotate horizontally by delta_yaw radians.
    pub fn rotate(&mut self, delta_yaw: f32) {
        self.yaw += delta_yaw;
    }
}

impl Default for TabletopCamera {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
// Matrix Math (no external deps)
// ──────────────────────────────────────────────

/// Standard look-at view matrix.
fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize3([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let s = normalize3(cross3(f, up));
    let u = cross3(s, f);

    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [
            -(s[0] * eye[0] + s[1] * eye[1] + s[2] * eye[2]),
            -(u[0] * eye[0] + u[1] * eye[1] + u[2] * eye[2]),
            -(-f[0] * eye[0] + -f[1] * eye[1] + -f[2] * eye[2]),
            1.0,
        ],
    ]
}

/// Perspective projection matrix (column-major).
fn perspective(fov: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fov * 0.5).tan();
    let range = far - near;

    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, -(far + near) / range, -1.0],
        [0.0, 0.0, -(2.0 * far * near) / range, 0.0],
    ]
}

/// 3D cross product.
fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Normalize a 3D vector. Returns zero vector if length is near zero.
fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-6 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

/// 4x4 matrix multiplication.
fn mat4_mul_mat4(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = a[0][row] * b[col][0]
                + a[1][row] * b[col][1]
                + a[2][row] * b[col][2]
                + a[3][row] * b[col][3];
        }
    }
    out
}

/// Matrix-vector multiply (4x4 * vec4).
fn mat4_mul_vec4(m: [[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

/// Invert a 4x4 matrix using cofactors.
fn invert_mat4(m: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let (m00, m01, m02, m03) = (m[0][0], m[0][1], m[0][2], m[0][3]);
    let (m10, m11, m12, m13) = (m[1][0], m[1][1], m[1][2], m[1][3]);
    let (m20, m21, m22, m23) = (m[2][0], m[2][1], m[2][2], m[2][3]);
    let (m30, m31, m32, m33) = (m[3][0], m[3][1], m[3][2], m[3][3]);

    let coef00 = m22 * m33 - m23 * m32;
    let coef02 = m12 * m33 - m13 * m32;
    let coef03 = m12 * m23 - m13 * m22;
    let coef04 = m21 * m33 - m23 * m31;
    let coef06 = m11 * m33 - m13 * m31;
    let coef07 = m11 * m23 - m13 * m21;
    let coef08 = m20 * m32 - m22 * m30;
    let coef10 = m10 * m32 - m12 * m30;
    let coef11 = m10 * m22 - m12 * m20;
    let coef12 = m21 * m32 - m22 * m31;
    let coef14 = m11 * m32 - m12 * m31;
    let coef15 = m11 * m22 - m12 * m21;
    let coef16 = m20 * m33 - m23 * m30;
    let coef18 = m10 * m33 - m13 * m30;
    let coef19 = m10 * m23 - m13 * m20;
    let coef20 = m20 * m31 - m21 * m30;
    let coef22 = m10 * m31 - m11 * m30;
    let coef23 = m10 * m21 - m11 * m20;

    let fac0 = [coef00, coef02, coef03];
    let fac1 = [coef04, coef06, coef07];
    let fac2 = [coef08, coef10, coef11];
    let fac3 = [coef12, coef14, coef15];
    let fac4 = [coef16, coef18, coef19];
    let fac5 = [coef20, coef22, coef23];

    let adj = [
        [
            m11 * fac0[0] - m21 * fac1[0] + m31 * fac2[0],
            -(m10 * fac0[0] - m20 * fac1[0] + m30 * fac2[0]),
            m10 * fac3[0] - m20 * fac4[0] + m30 * fac5[0],
            -(m10 * fac0[2] - m20 * fac1[2] + m30 * fac2[2]),
        ],
        [
            -(m01 * fac0[0] - m21 * fac3[0] + m31 * fac1[0]),
            m00 * fac0[0] - m20 * fac3[0] + m30 * fac1[0],
            -(m00 * fac3[0] - m20 * fac4[0] + m30 * fac5[0]),
            m00 * fac0[2] - m20 * fac1[2] + m30 * fac2[2],
        ],
        [
            m01 * fac1[0] - m11 * fac3[0] + m31 * fac5[0],
            -(m00 * fac1[0] - m10 * fac3[0] + m30 * fac5[0]),
            m00 * fac4[0] - m10 * fac4[0] + m30 * fac4[0],
            -(m00 * fac1[2] - m10 * fac3[2] + m30 * fac5[2]),
        ],
        [
            -(m01 * fac2[0] - m11 * fac4[0] + m21 * fac5[0]),
            m00 * fac2[0] - m10 * fac4[0] + m20 * fac5[0],
            -(m00 * fac3[0] - m10 * fac3[0] + m20 * fac5[0]),
            m00 * fac0[1] - m10 * fac1[1] + m20 * fac2[1],
        ],
    ];

    let det = m00 * adj[0][0] + m01 * adj[0][1] + m02 * adj[0][2] + m03 * adj[0][3];
    if det.abs() < 1e-10 {
        // Return identity if non-invertible
        return [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
    }

    let inv_det = 1.0 / det;
    let mut out = [[0.0f32; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            out[col][row] = adj[col][row] * inv_det;
        }
    }
    out
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_construction() {
        let cam = TabletopCamera::new();
        assert_eq!(cam.target, [0.0, 0.0, 0.0]);
        assert!((cam.pitch - 0.85).abs() < 1e-4);
        assert!((cam.yaw).abs() < 1e-4);
        assert!((cam.distance - 20.0).abs() < 1e-4);
        assert!((cam.fov - std::f32::consts::FRAC_PI_4).abs() < 1e-4);
        assert!(cam.follow_target.is_none());
    }

    #[test]
    fn eye_position_spherical() {
        let cam = TabletopCamera::new();
        let eye = cam.eye_position();
        // Pitch 0.85 rad ≈ 48.7°, yaw 0.0, distance 20.0
        // eye_x = 0 + 20 * cos(0.85) * sin(0) = 0
        // eye_y = 0 + 20 * sin(0.85) ≈ 15.16
        // eye_z = 0 + 20 * cos(0.85) * cos(0) ≈ 13.12
        assert!(eye[0].abs() < 1e-4, "eye_x should be ~0, got {}", eye[0]);
        assert!(
            (eye[1] - 20.0 * 0.85_f32.sin()).abs() < 1e-4,
            "eye_y should be {}, got {}",
            20.0 * 0.85_f32.sin(),
            eye[1]
        );
        assert!(
            (eye[2] - 20.0 * 0.85_f32.cos()).abs() < 1e-4,
            "eye_z should be {}, got {}",
            20.0 * 0.85_f32.cos(),
            eye[2]
        );
    }

    #[test]
    fn zoom_clamping() {
        let mut cam = TabletopCamera::new();
        cam.min_distance = 5.0;
        cam.max_distance = 100.0;

        cam.zoom(200.0);
        assert!(
            (cam.distance - 100.0).abs() < 1e-4,
            "distance should clamp to max: {}",
            cam.distance
        );

        cam.zoom(-200.0);
        assert!(
            (cam.distance - 5.0).abs() < 1e-4,
            "distance should clamp to min: {}",
            cam.distance
        );
    }

    #[test]
    fn pan_moves_target() {
        let mut cam = TabletopCamera::new();
        let original = cam.target;
        cam.pan(10.0, 5.0, 1.0);
        assert!(
            (cam.target[0] - (original[0] + 10.0)).abs() < 1e-4,
            "target x should shift by dx*dt"
        );
        assert!(
            (cam.target[2] - (original[2] + 5.0)).abs() < 1e-4,
            "target z should shift by dz*dt"
        );
    }

    #[test]
    fn follow_smoothing() {
        let mut cam = TabletopCamera::new();
        cam.follow_target = Some([10.0, 0.0, 10.0]);
        cam.follow_smoothing = 5.0;

        cam.update(1.0);
        // After 1 second with smoothing=5.0, t = min(5.0*1.0, 1.0) = 1.0
        // So target should snap to follow_target
        assert!(
            (cam.target[0] - 10.0).abs() < 1e-4,
            "target x should be at follow: {}",
            cam.target[0]
        );
        assert!(
            (cam.target[2] - 10.0).abs() < 1e-4,
            "target z should be at follow: {}",
            cam.target[2]
        );

        // Partial smoothing
        cam.target = [0.0, 0.0, 0.0];
        cam.follow_target = Some([10.0, 0.0, 0.0]);
        cam.follow_smoothing = 0.5;
        cam.update(1.0);
        // t = min(0.5*1.0, 1.0) = 0.5
        // target = 0 + (10 - 0) * 0.5 = 5.0
        assert!(
            (cam.target[0] - 5.0).abs() < 1e-4,
            "target x should be 5.0, got {}",
            cam.target[0]
        );
    }

    #[test]
    fn screen_to_ray_nonzero_direction() {
        let cam = TabletopCamera::new();
        let (origin, dir) = cam.screen_to_ray(400.0, 300.0, 800.0, 600.0);
        // Direction should be non-zero
        let len_sq = dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2];
        assert!(
            len_sq > 0.01,
            "ray direction should be non-zero, len_sq={}",
            len_sq
        );
        // Origin should be somewhere near the camera
        assert!(
            origin[1] > -0.1,
            "ray origin y should be near or above ground, got {}",
            origin[1]
        );
    }

    #[test]
    fn rotate_changes_yaw() {
        let mut cam = TabletopCamera::new();
        let original_yaw = cam.yaw;
        cam.rotate(std::f32::consts::FRAC_PI_4);
        assert!(
            (cam.yaw - (original_yaw + std::f32::consts::FRAC_PI_4)).abs() < 1e-4,
            "yaw should increase by pi/4"
        );
    }
}
