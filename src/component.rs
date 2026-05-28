/// A marker trait for ECS component types.
///
/// Any type that implements `Component` can be stored in the ECS.
/// Implementation is automatic for any `Send + Sync + 'static` type
/// via a blanket impl — no boilerplate needed.
pub trait Component: Send + Sync + 'static {}

// Blanket implementation: anything that is Send + Sync + 'static is a valid component.
impl<T: Send + Sync + 'static> Component for T {}

/// Example: a 2D position component.
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Position { x, y }
    }
}

/// Example: a velocity component.
#[derive(Debug, Clone, Copy, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

impl Velocity {
    pub fn new(x: f32, y: f32) -> Self {
        Velocity { x, y }
    }
}

/// Example: a health component.
#[derive(Debug, Clone, Default)]
pub struct Health {
    pub current: u32,
    pub max: u32,
}

impl Health {
    pub fn new(max: u32) -> Self {
        Health {
            current: max,
            max,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.current == 0
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.current = self.current.saturating_sub(amount);
    }
}

/// A tag component for damage dealing.
#[derive(Debug, Clone, Default)]
pub struct Damage(pub u32);

/// A tag component to mark entities as dead.
#[derive(Debug, Clone, Default)]
pub struct Dead;

/// A transform component (position + rotation + scale) for 3D-capable systems.
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rotation: f32,
    pub scale: f32,
}

impl Transform {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Transform {
            x,
            y,
            z,
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

/// A visual representation component (sprite/render hint).
#[derive(Debug, Clone, Default)]
pub struct Sprite {
    pub symbol: char,
    pub color: (u8, u8, u8),
    pub layer: i32,
}

impl Sprite {
    pub fn new(symbol: char, r: u8, g: u8, b: u8) -> Self {
        Sprite {
            symbol,
            color: (r, g, b),
            layer: 0,
        }
    }
}

/// Circle collision radius for narrow-phase checks.
#[derive(Debug, Clone, Copy, Default)]
pub struct CircleRadius(pub f32);

impl CircleRadius {
    pub fn new(radius: f32) -> Self {
        CircleRadius(radius)
    }
}

/// Rigid body with mass and restitution.
#[derive(Debug, Clone, Copy, Default)]
pub struct RigidBody {
    pub mass: f32,
    pub damping: f32,
    pub restitution: f32,
}

impl RigidBody {
    pub fn new(mass: f32, damping: f32, restitution: f32) -> Self {
        RigidBody {
            mass,
            damping,
            restitution,
        }
    }

    pub fn static_body() -> Self {
        RigidBody {
            mass: 0.0,
            damping: 0.0,
            restitution: 0.0,
        }
    }

    pub fn kinematic() -> Self {
        RigidBody {
            mass: 0.0,
            damping: 0.0,
            restitution: 0.0,
        }
    }

    pub fn dynamic(mass: f32) -> Self {
        RigidBody {
            mass,
            damping: 0.99,
            restitution: 0.5,
        }
    }

    pub fn is_static(&self) -> bool {
        self.mass == 0.0
    }
}

/// Tag for entities on the ground.
#[derive(Debug, Clone, Copy, Default)]
pub struct Grounded;

/// Gravity direction component.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gravity {
    pub x: f32,
    pub y: f32,
}

impl Gravity {
    pub fn new(x: f32, y: f32) -> Self {
        Gravity { x, y }
    }

    pub fn down(acceleration: f32) -> Self {
        Gravity {
            x: 0.0,
            y: acceleration,
        }
    }
}
