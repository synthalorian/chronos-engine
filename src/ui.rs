//! UI widget primitives with hit-testing and layout.
//!
//! Provides immediate-mode-style widgets rendered as colored quads
//! via the sprite batch system.

use crate::render::RenderSprite;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WidgetState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WidgetStyle {
    pub normal_color: [f32; 4],
    pub hovered_color: [f32; 4],
    pub pressed_color: [f32; 4],
    pub disabled_color: [f32; 4],
    pub border_color: [f32; 4],
    pub text_color: [f32; 4],
    pub border_thickness: f32,
}

impl WidgetStyle {
    pub fn dark() -> Self {
        WidgetStyle {
            normal_color: [0.15, 0.15, 0.2, 0.9],
            hovered_color: [0.25, 0.25, 0.35, 0.95],
            pressed_color: [0.1, 0.1, 0.15, 1.0],
            disabled_color: [0.1, 0.1, 0.1, 0.5],
            border_color: [0.4, 0.4, 0.5, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            border_thickness: 2.0,
        }
    }

    pub fn light() -> Self {
        WidgetStyle {
            normal_color: [0.85, 0.85, 0.9, 0.9],
            hovered_color: [0.9, 0.9, 0.95, 0.95],
            pressed_color: [0.7, 0.7, 0.75, 1.0],
            disabled_color: [0.6, 0.6, 0.6, 0.5],
            border_color: [0.3, 0.3, 0.35, 1.0],
            text_color: [0.1, 0.1, 0.15, 1.0],
            border_thickness: 2.0,
        }
    }

    pub fn accent() -> Self {
        WidgetStyle {
            normal_color: [0.2, 0.4, 0.8, 0.9],
            hovered_color: [0.3, 0.5, 0.9, 0.95],
            pressed_color: [0.15, 0.3, 0.7, 1.0],
            disabled_color: [0.1, 0.15, 0.3, 0.5],
            border_color: [0.4, 0.6, 1.0, 1.0],
            text_color: [1.0, 1.0, 1.0, 1.0],
            border_thickness: 2.0,
        }
    }

    pub fn color_for_state(&self, state: WidgetState) -> [f32; 4] {
        match state {
            WidgetState::Normal => self.normal_color,
            WidgetState::Hovered => self.hovered_color,
            WidgetState::Pressed => self.pressed_color,
            WidgetState::Disabled => self.disabled_color,
        }
    }
}

pub struct Button {
    pub rect: Rect,
    pub label: String,
    pub style: WidgetStyle,
    pub state: WidgetState,
    pub layer: i32,
    pub clicked: bool,
}

impl Button {
    pub fn new(x: f32, y: f32, w: f32, h: f32, label: &str) -> Self {
        Button {
            rect: Rect::new(x, y, w, h),
            label: label.to_string(),
            style: WidgetStyle::accent(),
            state: WidgetState::Normal,
            layer: 1000,
            clicked: false,
        }
    }

    pub fn with_style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn update(&mut self, mouse_x: f32, mouse_y: f32, mouse_down: bool, mouse_clicked: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }

        let hovered = self.rect.contains(mouse_x, mouse_y);
        self.state = if hovered {
            if mouse_down {
                WidgetState::Pressed
            } else {
                WidgetState::Hovered
            }
        } else {
            WidgetState::Normal
        };

        self.clicked = hovered && mouse_clicked;
    }

    pub fn to_sprites(&self) -> Vec<RenderSprite> {
        let mut sprites = Vec::new();
        let color = self.style.color_for_state(self.state);

        // Background fill
        sprites.push(
            RenderSprite::new(
                self.rect.x + self.rect.w * 0.5,
                self.rect.y + self.rect.h * 0.5,
                self.rect.w,
                self.rect.h,
            )
            .with_color(color[0], color[1], color[2], color[3])
            .with_layer(self.layer),
        );

        // Border (4 thin quads)
        let bt = self.style.border_thickness;
        let bc = self.style.border_color;
        let (cx, cy) = self.rect.center();

        // Top border
        sprites.push(
            RenderSprite::new(cx, self.rect.y - bt * 0.5, self.rect.w + bt * 2.0, bt)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        // Bottom border
        sprites.push(
            RenderSprite::new(cx, self.rect.y + self.rect.h + bt * 0.5, self.rect.w + bt * 2.0, bt)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        // Left border
        sprites.push(
            RenderSprite::new(self.rect.x - bt * 0.5, cy, bt, self.rect.h)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        // Right border
        sprites.push(
            RenderSprite::new(self.rect.x + self.rect.w + bt * 0.5, cy, bt, self.rect.h)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );

        sprites
    }
}

pub struct Slider {
    pub rect: Rect,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub style: WidgetStyle,
    pub state: WidgetState,
    pub layer: i32,
    pub dragging: bool,
    pub handle_width: f32,
}

impl Slider {
    pub fn new(x: f32, y: f32, w: f32, h: f32, min: f32, max: f32, value: f32) -> Self {
        Slider {
            rect: Rect::new(x, y, w, h),
            value: value.clamp(min, max),
            min,
            max,
            style: WidgetStyle::dark(),
            state: WidgetState::Normal,
            layer: 1000,
            dragging: false,
            handle_width: 12.0,
        }
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn normalized(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            return 0.0;
        }
        (self.value - self.min) / (self.max - self.min)
    }

    pub fn handle_rect(&self) -> Rect {
        let t = self.normalized();
        let track_w = self.rect.w - self.handle_width;
        let hx = self.rect.x + t * track_w;
        Rect::new(hx, self.rect.y, self.handle_width, self.rect.h)
    }

    pub fn update(&mut self, mouse_x: f32, mouse_y: f32, mouse_down: bool) {
        if self.state == WidgetState::Disabled {
            return;
        }

        if self.dragging && mouse_down {
            let t = ((mouse_x - self.rect.x) / (self.rect.w - self.handle_width))
                .clamp(0.0, 1.0);
            self.value = self.min + t * (self.max - self.min);
            return;
        }

        self.dragging = false;
        let hovered = self.rect.contains(mouse_x, mouse_y);
        self.state = if hovered && mouse_down {
            self.dragging = true;
            WidgetState::Pressed
        } else if hovered {
            WidgetState::Hovered
        } else {
            WidgetState::Normal
        };
    }

    pub fn to_sprites(&self) -> Vec<RenderSprite> {
        let mut sprites = Vec::new();
        let (cx, cy) = self.rect.center();

        // Track background
        sprites.push(
            RenderSprite::new(cx, cy, self.rect.w, self.rect.h)
                .with_color(0.2, 0.2, 0.25, 0.9)
                .with_layer(self.layer),
        );

        // Filled portion
        let t = self.normalized();
        let fill_w = t * (self.rect.w - self.handle_width);
        if fill_w > 0.0 {
            let fc = self.style.color_for_state(self.state);
            sprites.push(
                RenderSprite::new(
                    self.rect.x + fill_w * 0.5,
                    cy,
                    fill_w,
                    self.rect.h,
                )
                .with_color(fc[0], fc[1], fc[2], fc[3])
                .with_layer(self.layer + 1),
            );
        }

        // Handle
        let handle = self.handle_rect();
        let hc = self.style.color_for_state(if self.dragging {
            WidgetState::Pressed
        } else {
            self.state
        });
        sprites.push(
            RenderSprite::new(
                handle.x + handle.w * 0.5,
                handle.y + handle.h * 0.5,
                handle.w,
                handle.h,
            )
            .with_color(hc[0], hc[1], hc[2], hc[3])
            .with_layer(self.layer + 2),
        );

        sprites
    }
}

pub struct Panel {
    pub rect: Rect,
    pub style: WidgetStyle,
    pub layer: i32,
    pub visible: bool,
}

impl Panel {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Panel {
            rect: Rect::new(x, y, w, h),
            style: WidgetStyle::dark(),
            layer: 900,
            visible: true,
        }
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn to_sprites(&self) -> Vec<RenderSprite> {
        if !self.visible {
            return Vec::new();
        }

        let mut sprites = Vec::new();
        let (cx, cy) = self.rect.center();

        sprites.push(
            RenderSprite::new(cx, cy, self.rect.w, self.rect.h)
                .with_color(
                    self.style.normal_color[0],
                    self.style.normal_color[1],
                    self.style.normal_color[2],
                    self.style.normal_color[3],
                )
                .with_layer(self.layer),
        );

        let bt = self.style.border_thickness;
        let bc = self.style.border_color;

        sprites.push(
            RenderSprite::new(cx, self.rect.y - bt * 0.5, self.rect.w + bt * 2.0, bt)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        sprites.push(
            RenderSprite::new(cx, self.rect.y + self.rect.h + bt * 0.5, self.rect.w + bt * 2.0, bt)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        sprites.push(
            RenderSprite::new(self.rect.x - bt * 0.5, cy, bt, self.rect.h)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );
        sprites.push(
            RenderSprite::new(self.rect.x + self.rect.w + bt * 0.5, cy, bt, self.rect.h)
                .with_color(bc[0], bc[1], bc[2], bc[3])
                .with_layer(self.layer + 1),
        );

        sprites
    }
}

pub struct Label {
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
    pub layer: i32,
    pub scale: f32,
}

impl Label {
    pub fn new(x: f32, y: f32) -> Self {
        Label {
            x,
            y,
            color: [1.0, 1.0, 1.0, 1.0],
            layer: 1001,
            scale: 1.0,
        }
    }

    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    pub fn to_sprites(
        &self,
        text: &str,
        font: &crate::font::BitmapFont,
        atlas: &crate::texture::TextureAtlas,
    ) -> Vec<RenderSprite> {
        font.render_text(text, self.x, self.y, self.scale, self.color, self.layer, atlas)
    }
}

pub struct UiContext {
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub mouse_down: bool,
    pub mouse_clicked: bool,
    pub screen_w: f32,
    pub screen_h: f32,
}

impl UiContext {
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        UiContext {
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: false,
            mouse_clicked: false,
            screen_w,
            screen_h,
        }
    }

    pub fn update_mouse(&mut self, x: f32, y: f32, down: bool, clicked: bool) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_down = down;
        self.mouse_clicked = clicked;
    }

    pub fn button(&mut self, id: usize, x: f32, y: f32, w: f32, h: f32, label: &str) -> bool {
        let _ = id;
        let mut btn = Button::new(x, y, w, h, label);
        btn.update(self.mouse_x, self.mouse_y, self.mouse_down, self.mouse_clicked);
        btn.clicked
    }
}
