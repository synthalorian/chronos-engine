//! Chronos Game — Playable Chronos Company RPG.
//!
//! A standalone windowed game that runs the full Chronos Company simulation
//! with wgpu rendering and mouse/keyboard input.

use chronos_engine::{
    Camera, InputEvent, InputManager, KeyCode, MouseButton, Position, RenderSprite, Renderer,
    SpriteBatch,
};
#[cfg(feature = "game")]
use chronos_engine::game::runner::{ChronosCompanyGame, GameConfig, GameMode};

use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

// ── Game App ───────────────────────────────────────────────────────────────

struct GameApp {
    window: Arc<Window>,
    renderer: Option<Renderer>,
    camera: Camera,
    sprite_batch: Option<SpriteBatch>,
    white_texture: Option<wgpu::Texture>,
    white_view: Option<wgpu::TextureView>,
    sampler: Option<wgpu::Sampler>,

    #[cfg(feature = "game")]
    game: Option<ChronosCompanyGame>,

    input: InputManager,
    last_frame: std::time::Instant,
    camera_pan: [f32; 2],
    camera_zoom: f32,
    mouse_world_pos: [f32; 2],
}

impl GameApp {
    async fn new(event_loop: &EventLoop<()>) -> Result<Self, String> {
        let window_attrs = Window::default_attributes()
            .with_title("Chronos Company")
            .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .map_err(|e| format!("Window creation failed: {e}"))?,
        );

        let mut renderer = Renderer::new(window.clone(), 1280, 720)
            .await
            .map_err(|e| format!("Renderer init failed: {e}"))?;

        let sprite_batch = SpriteBatch::new(&renderer.device, 4096);

        // 1x1 white texture
        let white_tex = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("white"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        renderer.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &white_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let white_view = white_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let camera = Camera::new(1280.0, 720.0);

        #[cfg(feature = "game")]
        let game = {
            let config = GameConfig {
                mode: GameMode::Campaign,
                map_width: 40,
                map_height: 40,
                world_seed: 42,
                start_gold: 500,
                start_squad_size: 4,
                auto_save_interval: 300,
                screen_width: 1280.0,
                screen_height: 720.0,
                cell_size: 2.0,
            };
            let mut g = ChronosCompanyGame::new(config);
            g.new_game();
            Some(g)
        };

        let mut input = InputManager::new();
        let mut gameplay = chronos_engine::input::InputContext::new("gameplay");
        gameplay = gameplay.bind("move_up", chronos_engine::input::Binding::single(chronos_engine::input::InputSource::Key(KeyCode::W)));
        gameplay = gameplay.bind("move_down", chronos_engine::input::Binding::single(chronos_engine::input::InputSource::Key(KeyCode::S)));
        gameplay = gameplay.bind("move_left", chronos_engine::input::Binding::single(chronos_engine::input::InputSource::Key(KeyCode::A)));
        gameplay = gameplay.bind("move_right", chronos_engine::input::Binding::single(chronos_engine::input::InputSource::Key(KeyCode::D)));
        gameplay = gameplay.bind("select", chronos_engine::input::Binding::single(chronos_engine::input::InputSource::Mouse(MouseButton::Left)));
        input.add_context(gameplay);
        input.set_context("gameplay");

        Ok(Self {
            window,
            renderer: Some(renderer),
            camera,
            sprite_batch: Some(sprite_batch),
            white_texture: Some(white_tex),
            white_view: Some(white_view),
            sampler: Some(sampler),
            #[cfg(feature = "game")]
            game,
            input,
            last_frame: std::time::Instant::now(),
            camera_pan: [0.0, 0.0],
            camera_zoom: 1.0,
            mouse_world_pos: [0.0, 0.0],
        })
    }

    fn screen_to_world(&self, sx: f32, sy: f32) -> [f32; 2] {
        let size = self.window.inner_size();
        let hw = size.width as f32 / 2.0;
        let hh = size.height as f32 / 2.0;
        let zoom = self.camera.zoom;
        [
            self.camera.position[0] + (sx - hw) / zoom,
            self.camera.position[1] + (sy - hh) / zoom,
        ]
    }

    fn update(&mut self, dt: f32) {
        // Camera pan
        let pan_speed = 200.0 * dt;
        if self.input.pressed("move_up") {
            self.camera_pan[1] -= pan_speed;
        }
        if self.input.pressed("move_down") {
            self.camera_pan[1] += pan_speed;
        }
        if self.input.pressed("move_left") {
            self.camera_pan[0] -= pan_speed;
        }
        if self.input.pressed("move_right") {
            self.camera_pan[0] += pan_speed;
        }
        self.camera.position[0] += self.camera_pan[0] * dt;
        self.camera.position[1] += self.camera_pan[1] * dt;
        self.camera_pan[0] *= 0.9;
        self.camera_pan[1] *= 0.9;
        self.camera.zoom = self.camera_zoom;

        // Game tick
        #[cfg(feature = "game")]
        if let Some(game) = &mut self.game {
            game.tick(dt as f64);
        }

        self.input.end_frame();
    }

    fn gather_sprites(&self) -> Vec<RenderSprite> {
        let mut sprites = Vec::new();

        #[cfg(feature = "game")]
        if let Some(game) = &self.game {
            // Render terrain grid
            let cell_size = game.config.cell_size * 20.0;
            let map_w = game.config.map_width as f32 * cell_size;
            let map_h = game.config.map_height as f32 * cell_size;
            let offset_x = -map_w / 2.0;
            let offset_y = -map_h / 2.0;

            for y in 0..game.config.map_height {
                for x in 0..game.config.map_width {
                    let px = offset_x + x as f32 * cell_size;
                    let py = offset_y + y as f32 * cell_size;
                    let color = if (x + y) % 2 == 0 {
                        [0.15, 0.18, 0.12, 1.0]
                    } else {
                        [0.12, 0.15, 0.10, 1.0]
                    };
                    sprites.push(
                        RenderSprite::new(px, py, cell_size - 1.0, cell_size - 1.0)
                            .with_color(color[0], color[1], color[2], color[3]),
                    );
                }
            }

            // Render squad
            for entity in &game.squad_entities {
                if let Some(pos) = game.world.get_component::<Position>(*entity) {
                    let px = offset_x + pos.x * 20.0;
                    let py = offset_y + pos.y * 20.0;
                    sprites.push(
                        RenderSprite::new(px, py, 14.0, 14.0)
                            .with_color(0.2, 0.6, 1.0, 1.0),
                    );
                }
            }

            // Render player entity
            if let Some(player) = game.player_entity {
                if let Some(pos) = game.world.get_component::<Position>(player) {
                    let px = offset_x + pos.x * 20.0;
                    let py = offset_y + pos.y * 20.0;
                    sprites.push(
                        RenderSprite::new(px, py, 18.0, 18.0)
                            .with_color(1.0, 0.8, 0.2, 1.0),
                    );
                }
            }
        }

        sprites
    }

    fn render(&mut self) {
        let sprites = self.gather_sprites();
        let Some(renderer) = self.renderer.as_mut() else { return };
        let Some(view) = self.white_view.as_ref() else { return };
        let Some(sampler) = self.sampler.as_ref() else { return };
        renderer.render(&self.camera, &mut sprites.clone(), view, sampler);
    }

    fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                use winit::keyboard::{KeyCode as WKey, PhysicalKey};
                if let PhysicalKey::Code(code) = event.physical_key {
                    let key = winit_to_chronos_key(code);
                    if event.state.is_pressed() {
                        self.input.process_event(&InputEvent::KeyPressed(key));
                    } else {
                        self.input.process_event(&InputEvent::KeyReleased(key));
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = match button {
                    winit::event::MouseButton::Left => MouseButton::Left,
                    winit::event::MouseButton::Right => MouseButton::Right,
                    winit::event::MouseButton::Middle => MouseButton::Middle,
                    _ => MouseButton::Left,
                };
                if state.is_pressed() {
                    self.input.process_event(&InputEvent::MousePressed(btn));
                } else {
                    self.input.process_event(&InputEvent::MouseReleased(btn));
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32;
                let y = position.y as f32;
                self.mouse_world_pos = self.screen_to_world(x, y);
                self.input.process_event(&InputEvent::MouseMoved {
                    x: position.x,
                    y: position.y,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 120.0,
                };
                self.camera_zoom = (self.camera_zoom + dy * 0.1).clamp(0.2, 5.0);
                self.input.process_event(&InputEvent::MouseScroll {
                    delta_y: dy,
                    delta_x: 0.0,
                });
            }
            _ => {}
        }
    }
}

// ── winit → chronos key mapping ────────────────────────────────────────────

fn winit_to_chronos_key(code: winit::keyboard::KeyCode) -> KeyCode {
    use winit::keyboard::KeyCode as W;
    match code {
        W::KeyA => KeyCode::A,
        W::KeyB => KeyCode::B,
        W::KeyC => KeyCode::C,
        W::KeyD => KeyCode::D,
        W::KeyE => KeyCode::E,
        W::KeyF => KeyCode::F,
        W::KeyG => KeyCode::G,
        W::KeyH => KeyCode::H,
        W::KeyI => KeyCode::I,
        W::KeyJ => KeyCode::J,
        W::KeyK => KeyCode::K,
        W::KeyL => KeyCode::L,
        W::KeyM => KeyCode::M,
        W::KeyN => KeyCode::N,
        W::KeyO => KeyCode::O,
        W::KeyP => KeyCode::P,
        W::KeyQ => KeyCode::Q,
        W::KeyR => KeyCode::R,
        W::KeyS => KeyCode::S,
        W::KeyT => KeyCode::T,
        W::KeyU => KeyCode::U,
        W::KeyV => KeyCode::V,
        W::KeyW => KeyCode::W,
        W::KeyX => KeyCode::X,
        W::KeyY => KeyCode::Y,
        W::KeyZ => KeyCode::Z,
        W::Digit0 => KeyCode::Key0,
        W::Digit1 => KeyCode::Key1,
        W::Digit2 => KeyCode::Key2,
        W::Digit3 => KeyCode::Key3,
        W::Digit4 => KeyCode::Key4,
        W::Digit5 => KeyCode::Key5,
        W::Digit6 => KeyCode::Key6,
        W::Digit7 => KeyCode::Key7,
        W::Digit8 => KeyCode::Key8,
        W::Digit9 => KeyCode::Key9,
        W::Space => KeyCode::Space,
        W::Enter => KeyCode::Return,
        W::Escape => KeyCode::Escape,
        W::ShiftLeft => KeyCode::LShift,
        W::ShiftRight => KeyCode::RShift,
        W::ControlLeft => KeyCode::LCtrl,
        W::ControlRight => KeyCode::RCtrl,
        W::AltLeft => KeyCode::LAlt,
        W::AltRight => KeyCode::RAlt,
        W::Tab => KeyCode::Tab,
        W::Backspace => KeyCode::Backspace,
        W::ArrowUp => KeyCode::Up,
        W::ArrowDown => KeyCode::Down,
        W::ArrowLeft => KeyCode::Left,
        W::ArrowRight => KeyCode::Right,
        W::F1 => KeyCode::F1,
        W::F2 => KeyCode::F2,
        W::F3 => KeyCode::F3,
        W::F4 => KeyCode::F4,
        W::F5 => KeyCode::F5,
        W::F6 => KeyCode::F6,
        W::F7 => KeyCode::F7,
        W::F8 => KeyCode::F8,
        W::F9 => KeyCode::F9,
        W::F10 => KeyCode::F10,
        W::F11 => KeyCode::F11,
        W::F12 => KeyCode::F12,
        _ => KeyCode::Space,
    }
}

// ── Application Handler ────────────────────────────────────────────────────

struct GameAppHandler {
    app: Option<GameApp>,
}

impl GameAppHandler {
    fn new(event_loop: &EventLoop<()>) -> Result<Self, String> {
        let app = pollster::block_on(GameApp::new(event_loop))?;
        Ok(Self { app: Some(app) })
    }
}

impl ApplicationHandler for GameAppHandler {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(app) = self.app.as_mut() else { return };

        match &event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    app.camera.viewport_width = size.width as f32;
                    app.camera.viewport_height = size.height as f32;
                    if let Some(renderer) = app.renderer.as_mut() {
                        renderer.resize(size.width, size.height);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = (now - app.last_frame).as_secs_f32().min(0.1);
                app.last_frame = now;
                app.update(dt);
                app.render();
            }
            _ => {
                app.handle_event(&event);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_ref() {
            app.window.request_redraw();
        }
    }
}

// ── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut handler = GameAppHandler::new(&event_loop).expect("Failed to init game");
    event_loop.run_app(&mut handler).expect("Event loop failed");
}
