//! Skeletal animation system with skeleton hierarchies, keyframe animation, and blend support.

#[derive(Debug, Clone, Copy)]
pub struct JointPose {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl JointPose {
    pub fn identity() -> Self {
        JointPose {
            translation: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 1.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    pub fn with_translation(mut self, x: f32, y: f32, z: f32) -> Self {
        self.translation = [x, y, z];
        self
    }

    pub fn to_matrix(&self) -> [[f32; 4]; 4] {
        mat4_from_trs(self.translation, self.rotation, self.scale)
    }
}

#[derive(Debug, Clone)]
pub struct Joint {
    pub name: String,
    pub parent_index: Option<usize>,
    pub inverse_bind_pose: [[f32; 4]; 4],
    pub local_bind_pose: JointPose,
}

#[derive(Debug, Clone)]
pub struct Skeleton {
    joints: Vec<Joint>,
}

impl Skeleton {
    pub fn new() -> Self {
        Skeleton { joints: Vec::new() }
    }

    pub fn add_joint(&mut self, name: &str, parent: Option<usize>, local_bind: JointPose) -> usize {
        let idx = self.joints.len();
        let inverse = local_bind.to_matrix();
        let inverse = if let Some(p) = parent {
            mat4_multiply(inverse, self.joints[p].inverse_bind_pose)
        } else {
            mat4_inverse(inverse)
        };
        let joint = Joint {
            name: name.to_string(),
            parent_index: parent,
            inverse_bind_pose: inverse,
            local_bind_pose: local_bind,
        };
        self.joints.push(joint);
        idx
    }

    pub fn joint_count(&self) -> usize {
        self.joints.len()
    }

    pub fn get_joint(&self, index: usize) -> Option<&Joint> {
        self.joints.get(index)
    }

    pub fn root_joints(&self) -> Vec<usize> {
        self.joints.iter()
            .enumerate()
            .filter(|(_, j)| j.parent_index.is_none())
            .map(|(i, _)| i)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct SkeletonPose {
    joint_poses: Vec<JointPose>,
}

impl SkeletonPose {
    pub fn new(joint_count: usize) -> Self {
        let joint_poses = (0..joint_count)
            .map(|_| JointPose {
                translation: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 1.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            })
            .collect();
        SkeletonPose { joint_poses }
    }

    pub fn set_joint_pose(&mut self, joint_index: usize, pose: JointPose) {
        self.joint_poses[joint_index] = pose;
    }

    pub fn get_joint_pose(&self, joint_index: usize) -> Option<&JointPose> {
        self.joint_poses.get(joint_index)
    }

    pub fn compute_skin_matrices(&self, skeleton: &Skeleton) -> Vec<[[f32; 4]; 4]> {
        let count = self.joint_poses.len();
        let mut skin_matrices = Vec::with_capacity(count);
        let mut parent_world: Vec<[[f32; 4]; 4]> = Vec::with_capacity(count);

        for i in 0..count {
            let local = self.joint_poses[i].to_matrix();
            let world = if let Some(parent_idx) = skeleton.joints[i].parent_index {
                mat4_multiply(parent_world[parent_idx], local)
            } else {
                local
            };
            parent_world.push(world);
            skin_matrices.push(parent_world[i]);
        }
        skin_matrices
    }
}

#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub joint_index: usize,
    pub translations: Vec<(f32, [f32; 3])>,
    pub rotations: Vec<(f32, [f32; 4])>,
    pub scales: Vec<(f32, [f32; 3])>,
}

impl AnimationChannel {
    pub fn new(joint_index: usize) -> Self {
        AnimationChannel {
            joint_index,
            translations: Vec::new(),
            rotations: Vec::new(),
            scales: Vec::new(),
        }
    }

    pub fn add_translation(&mut self, time: f32, value: [f32; 3]) {
        self.translations.push((time, value));
    }

    pub fn add_rotation(&mut self, time: f32, value: [f32; 4]) {
        self.rotations.push((time, value));
    }

    pub fn add_scale(&mut self, time: f32, value: [f32; 3]) {
        self.scales.push((time, value));
    }
}

#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
}

impl AnimationClip {
    pub fn new(name: &str, duration: f32) -> Self {
        AnimationClip {
            name: name.to_string(),
            duration,
            channels: Vec::new(),
        }
    }

    pub fn add_channel(&mut self, channel: AnimationChannel) {
        self.channels.push(channel);
    }
}

#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    clip: Option<AnimationClip>,
    time: f32,
    playing: bool,
    looping: bool,
    speed: f32,
}

#[derive(Debug, Clone)]
pub struct AnimationBlender;

fn quat_normalize(q: [f32; 4]) -> [f32; 4] {
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len == 0.0 {
        return [0.0, 0.0, 1.0, 0.0];
    }
    [q[0] / len, q[1] / len, q[2] / len, q[3] / len]
}

#[allow(dead_code)]
fn quat_multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] + a[1] * b[3] + a[2] * b[0] - a[0] * b[2],
        a[3] * b[2] + a[2] * b[3] + a[0] * b[1] - a[1] * b[0],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn quat_slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

    let b = if dot < 0.0 {
        [-b[0], -b[1], -b[2], -b[3]]
    } else {
        b
    };

    let dot = dot.abs();

    if dot > 0.9995 {
        let q0 = [
            a[0],
            a[1],
            a[2],
            a[3],
        ];
        let q1 = b;
        let result = [
            (1.0 - t) * q0[0] + t * q1[0],
            (1.0 - t) * q0[1] + t * q1[1],
            (1.0 - t) * q0[2] + t * q1[2],
            (1.0 - t) * q0[3] + t * q1[3],
        ];
        quat_normalize(result)
    } else {
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let s0 = ((1.0 - t) * theta).sin() / sin_theta;
        let s1 = (t * theta).sin() / sin_theta;
        [
            s0 * a[0] + s1 * b[0],
            s0 * a[1] + s1 * b[1],
            s0 * a[2] + s1 * b[2],
            s0 * a[3] + s1 * b[3],
        ]
    }
}

fn vec3_lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn mat4_identity() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn mat4_multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut result = mat4_identity();
    for i in 0..4 {
        for j in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[i][k] * b[k][j];
            }
            result[i][j] = sum;
        }
    }
    result
}

fn mat4_from_trs(translation: [f32; 3], rotation: [f32; 4], scale: [f32; 3]) -> [[f32; 4]; 4] {
    let x = rotation[0];
    let y = rotation[1];
    let z = rotation[2];
    let w = rotation[3];

    let x2 = x + x;
    let y2 = y + y;
    let z2 = z + z;

    let xx = x * x2;
    let yy = y * y2;
    let zz = z * z2;
    let xy = x * y2;
    let xz = x * z2;
    let yz = y * z2;
    let wx = w * x2;
    let wy = w * y2;
    let wz = w * z2;

    let sx = scale[0];
    let sy = scale[1];
    let sz = scale[2];

    [
        [sx * (1.0 - (yy + zz)), sx * (xy + wz), sx * (xz - wy), translation[0]],
        [sy * (xy - wz), sy * (1.0 - (xx + zz)), sy * (yz + wx), translation[1]],
        [sz * (xz + wy), sz * (yz - wx), sz * (1.0 - (xx + yy)), translation[2]],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

#[allow(dead_code)]
fn quat_to_matrix(q: [f32; 4]) -> [[f32; 4]; 4] {
    let x = q[0];
    let y = q[1];
    let z = q[2];
    let w = q[3];

    let x2 = x + x;
    let y2 = y + y;
    let z2 = z + z;

    let xx = x * x2;
    let yy = y * y2;
    let zz = z * z2;
    let xy = x * y2;
    let xz = x * z2;
    let yz = y * z2;
    let wx = w * x2;
    let wy = w * y2;
    let wz = w * z2;

    [
        [1.0 - (yy + zz), xy + wz, xz - wy, 0.0],
        [xy - wz, 1.0 - (xx + zz), yz + wx, 0.0],
        [xz + wy, yz - wx, 1.0 - (xx + yy), 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn mat4_inverse(m: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut inv = [[0.0f32; 4]; 4];

    inv[0][0] = m[1][1] * m[2][2] * m[3][3] - m[1][1] * m[2][3] * m[3][2]
              - m[2][1] * m[1][2] * m[3][3] + m[2][1] * m[1][3] * m[3][2]
              + m[3][1] * m[1][2] * m[2][3] - m[3][1] * m[1][3] * m[2][2];

    inv[1][0] = -m[1][0] * m[2][2] * m[3][3] + m[1][0] * m[2][3] * m[3][2]
               + m[2][0] * m[1][2] * m[3][3] - m[2][0] * m[1][3] * m[3][2]
               - m[3][0] * m[1][2] * m[2][3] + m[3][0] * m[1][3] * m[2][2];

    inv[2][0] = m[1][0] * m[2][1] * m[3][3] - m[1][0] * m[2][3] * m[3][1]
              - m[2][0] * m[1][1] * m[3][3] + m[2][0] * m[1][3] * m[3][1]
              + m[3][0] * m[1][1] * m[2][3] - m[3][0] * m[1][3] * m[2][1];

    inv[3][0] = -m[1][0] * m[2][1] * m[3][2] + m[1][0] * m[2][2] * m[3][1]
               + m[2][0] * m[1][1] * m[3][2] - m[2][0] * m[1][2] * m[3][1]
               - m[3][0] * m[1][1] * m[2][2] + m[3][0] * m[1][2] * m[2][1];

    inv[0][1] = -m[0][1] * m[2][2] * m[3][3] + m[0][1] * m[2][3] * m[3][2]
               + m[2][1] * m[0][2] * m[3][3] - m[2][1] * m[0][3] * m[3][2]
               - m[3][1] * m[0][2] * m[2][3] + m[3][1] * m[0][3] * m[2][2];

    inv[1][1] = m[0][0] * m[2][2] * m[3][3] - m[0][0] * m[2][3] * m[3][2]
              - m[2][0] * m[0][2] * m[3][3] + m[2][0] * m[0][3] * m[3][2]
              + m[3][0] * m[0][2] * m[2][3] - m[3][0] * m[0][3] * m[2][2];

    inv[2][1] = -m[0][0] * m[2][1] * m[3][3] + m[0][0] * m[2][3] * m[3][1]
               + m[2][0] * m[0][1] * m[3][3] - m[2][0] * m[0][3] * m[3][1]
               - m[3][0] * m[0][1] * m[2][3] + m[3][0] * m[0][3] * m[2][1];

    inv[3][1] = m[0][0] * m[2][1] * m[3][2] - m[0][0] * m[2][2] * m[3][1]
              - m[2][0] * m[0][1] * m[3][2] + m[2][0] * m[0][2] * m[3][1]
              + m[3][0] * m[0][1] * m[2][2] - m[3][0] * m[0][2] * m[2][1];

    inv[0][2] = m[0][1] * m[1][2] * m[3][3] - m[0][1] * m[1][3] * m[3][2]
              - m[1][1] * m[0][2] * m[3][3] + m[1][1] * m[0][3] * m[3][2]
              + m[3][1] * m[0][2] * m[1][3] - m[3][1] * m[0][3] * m[1][2];

    inv[1][2] = -m[0][0] * m[1][2] * m[3][3] + m[0][0] * m[1][3] * m[3][2]
               + m[1][0] * m[0][2] * m[3][3] - m[1][0] * m[0][3] * m[3][2]
               - m[3][0] * m[0][2] * m[1][3] + m[3][0] * m[0][3] * m[1][2];

    inv[2][2] = m[0][0] * m[1][1] * m[3][3] - m[0][0] * m[1][3] * m[3][1]
              - m[1][0] * m[0][1] * m[3][3] + m[1][0] * m[0][3] * m[3][1]
              + m[3][0] * m[0][1] * m[1][3] - m[3][0] * m[0][3] * m[1][1];

    inv[3][2] = -m[0][0] * m[1][1] * m[3][2] + m[0][0] * m[1][2] * m[3][1]
               + m[1][0] * m[0][1] * m[3][2] - m[1][0] * m[0][2] * m[3][1]
               - m[3][0] * m[0][1] * m[1][2] + m[3][0] * m[0][2] * m[1][1];

    inv[0][3] = -m[0][1] * m[1][2] * m[2][3] + m[0][1] * m[1][3] * m[2][2]
               + m[1][1] * m[0][2] * m[2][3] - m[1][1] * m[0][3] * m[2][2]
               - m[2][1] * m[0][2] * m[1][3] + m[2][1] * m[0][3] * m[1][2];

    inv[1][3] = m[0][0] * m[1][2] * m[2][3] - m[0][0] * m[1][3] * m[2][2]
              - m[1][0] * m[0][2] * m[2][3] + m[1][0] * m[0][3] * m[2][2]
              + m[2][0] * m[0][2] * m[1][3] - m[2][0] * m[0][3] * m[1][2];

    inv[2][3] = -m[0][0] * m[1][1] * m[2][3] + m[0][0] * m[1][3] * m[2][1]
               + m[1][0] * m[0][1] * m[2][3] - m[1][0] * m[0][3] * m[2][1]
               - m[2][0] * m[0][1] * m[1][3] + m[2][0] * m[0][3] * m[1][1];

    inv[3][3] = m[0][0] * m[1][1] * m[2][2] - m[0][0] * m[1][2] * m[2][1]
              - m[1][0] * m[0][1] * m[2][2] + m[1][0] * m[0][2] * m[2][1]
              + m[2][0] * m[0][1] * m[1][2] - m[2][0] * m[0][2] * m[1][1];

    let det = m[0][0] * inv[0][0]
           + m[0][1] * inv[1][0]
           + m[0][2] * inv[2][0]
           + m[0][3] * inv[3][0];

    if det.abs() < 1e-8 {
        return mat4_identity();
    }

    let inv_det = 1.0 / det;
    for i in 0..4 {
        for j in 0..4 {
            inv[i][j] *= inv_det;
        }
    }

    inv
}

impl AnimationPlayer {
    pub fn new() -> Self {
        AnimationPlayer {
            clip: None,
            time: 0.0,
            playing: false,
            looping: false,
            speed: 1.0,
        }
    }

    pub fn play(&mut self, clip: AnimationClip) {
        self.clip = Some(clip);
        self.time = 0.0;
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
    }

    pub fn set_time(&mut self, time: f32) {
        self.time = time;
    }

    pub fn update(&mut self, dt: f32) {
        if !self.playing {
            return;
        }
        self.time += dt * self.speed;
        if let Some(ref clip) = self.clip {
            if self.time >= clip.duration {
                if self.looping {
                    self.time -= clip.duration;
                } else {
                    self.time = clip.duration;
                    self.playing = false;
                }
            }
        }
    }

    pub fn sample(&self, joint_count: usize) -> SkeletonPose {
        let mut pose = SkeletonPose::new(joint_count);
        if let Some(ref clip) = self.clip {
            for channel in &clip.channels {
                let t = self.time;
                let trans = sample_keyframes(&channel.translations, t, [0.0, 0.0, 0.0]);
                let rot = sample_keyframes_quat(&channel.rotations, t, [0.0, 0.0, 1.0, 0.0]);
                let scale = sample_keyframes(&channel.scales, t, [1.0, 1.0, 1.0]);
                pose.set_joint_pose(
                    channel.joint_index,
                    JointPose {
                        translation: trans,
                        rotation: rot,
                        scale,
                    },
                );
            }
        }
        pose
    }
}

fn sample_keyframes(keyframes: &[(f32, [f32; 3])], time: f32, default: [f32; 3]) -> [f32; 3] {
    if keyframes.is_empty() {
        return default;
    }
    if keyframes.len() == 1 {
        return keyframes[0].1;
    }

    if time <= keyframes[0].0 {
        return keyframes[0].1;
    }
    if time >= keyframes[keyframes.len() - 1].0 {
        return keyframes[keyframes.len() - 1].1;
    }

    let idx = keyframes.partition_point(|(t, _)| *t <= time);
    let idx = if idx == 0 { 1 } else { idx };
    let prev = &keyframes[idx - 1];
    let next = &keyframes[idx];
    let t = (time - prev.0) / (next.0 - prev.0);
    vec3_lerp(prev.1, next.1, t)
}

fn sample_keyframes_quat(keyframes: &[(f32, [f32; 4])], time: f32, default: [f32; 4]) -> [f32; 4] {
    if keyframes.is_empty() {
        return default;
    }
    if keyframes.len() == 1 {
        return keyframes[0].1;
    }

    if time <= keyframes[0].0 {
        return keyframes[0].1;
    }
    if time >= keyframes[keyframes.len() - 1].0 {
        return keyframes[keyframes.len() - 1].1;
    }

    let idx = keyframes.partition_point(|(t, _)| *t <= time);
    let idx = if idx == 0 { 1 } else { idx };
    let prev = &keyframes[idx - 1];
    let next = &keyframes[idx];
    let t = (time - prev.0) / (next.0 - prev.0);
    quat_slerp(prev.1, next.1, t)
}

impl AnimationBlender {
    pub fn new() -> Self {
        AnimationBlender
    }

    pub fn blend(pose_a: &SkeletonPose, pose_b: &SkeletonPose, weight: f32) -> SkeletonPose {
        let count = pose_a.joint_poses.len().max(pose_b.joint_poses.len());
        let w = weight.clamp(0.0, 1.0);
        let mut blended = SkeletonPose::new(count);

        for i in 0..count {
            let pa = &pose_a.joint_poses[i];
            let pb = &pose_b.joint_poses[i];

            let translation = vec3_lerp(pa.translation, pb.translation, w);
            let rotation = quat_slerp(pa.rotation, pb.rotation, w);
            let scale = vec3_lerp(pa.scale, pb.scale, w);

            blended.set_joint_pose(i, JointPose {
                translation,
                rotation,
                scale,
            });
        }

        blended
    }
}
