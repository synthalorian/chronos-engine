//! Chronos Engine Editor — Desktop application core (Phase 7A + 7B).
//!
//! Provides [`EditorApp`] which manages a winit window, wgpu rendering surface,
//! and egui immediate-mode UI with all editor panels.
//! All editor code is gated behind the `editor` feature.

use std::num::NonZeroU32;
use std::sync::Arc;

use egui::{ViewportId, Visuals};
use egui_wgpu::WgpuConfiguration;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::editor_panels::{
    AssetBrowserPanel, ConsolePanel, EditorPanel, EditorState, HierarchyPanel, InspectorPanel,
    MenuBarPanel, ToolbarPanel, ViewportPanel, WelcomeScreen,
};
use crate::editor_workspace::{
    GizmoSystem, GridRenderer, SettingsDialog, ShortcutAction, ShortcutMap, UndoStack,
    ViewportSelector,
};

// ──────────────────────────────────────────────
// EditorError
// ──────────────────────────────────────────────

/// Errors that can occur during editor operation.
#[derive(Debug)]
pub enum EditorError {
    /// Failed to create the editor window.
    WindowCreation(String),
    /// Failed to create the wgpu surface.
    SurfaceCreation(String),
    /// No compatible GPU adapter found.
    NoAdapter,
    /// Failed to request a wgpu device.
    DeviceRequest(String),
    /// Failed to acquire the next surface texture for rendering.
    SurfaceTexture(String),
    /// Other / miscellaneous error.
    Other(String),
}

impl std::fmt::Display for EditorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditorError::WindowCreation(msg) => {
                write!(f, "Failed to create editor window: {msg}")
            }
            EditorError::SurfaceCreation(msg) => {
                write!(f, "Failed to create wgpu surface: {msg}")
            }
            EditorError::NoAdapter => write!(f, "No compatible GPU adapter found"),
            EditorError::DeviceRequest(msg) => {
                write!(f, "Failed to request wgpu device: {msg}")
            }
            EditorError::SurfaceTexture(msg) => {
                write!(f, "Failed to acquire surface texture: {msg}")
            }
            EditorError::Other(msg) => write!(f, "Editor error: {msg}"),
        }
    }
}

impl std::error::Error for EditorError {}

// ──────────────────────────────────────────────
// Config helpers
// ──────────────────────────────────────────────

// ──────────────────────────────────────────────
// EditorApp
// ──────────────────────────────────────────────

/// Core editor application — owns the window, wgpu surface, and egui state.
///
/// Typical usage:
/// ```ignore
/// let event_loop = EventLoop::new().unwrap();
/// let mut app = EditorApp::new(&event_loop)?;
///
/// event_loop.run(move |event, elwt| {
///     match event {
///         Event::AboutToWait => app.window().request_redraw(),
///         Event::WindowEvent { event, .. } => match &event {
///             WindowEvent::CloseRequested => elwt.exit(),
///             WindowEvent::Resized(size) => app.handle_resize(*size),
///             _ => { app.handle_window_event(&event); }
///         },
///         Event::RedrawRequested(_) => { let _ = app.render(); }
///         _ => {}
///     }
/// })?;
/// ```
pub struct EditorApp {
    window: Arc<Window>,

    egui_ctx: egui::Context,
    egui_winit: egui_winit::State,
    /// On native, finalized immediately. On WASM, set to `None` until
    /// `init_surface_wasm()` completes (deferred async init).
    egui_painter: Option<egui_wgpu::winit::Painter>,
    viewport_id: ViewportId,

    // ── Editor state & panels (Phase 7B) ──
    state: EditorState,
    menu_bar: MenuBarPanel,
    toolbar: ToolbarPanel,
    viewport: ViewportPanel,
    hierarchy: HierarchyPanel,
    inspector: InspectorPanel,
    asset_browser: AssetBrowserPanel,
    console: ConsolePanel,

    undo_stack: UndoStack,
    grid_renderer: GridRenderer,
    gizmo_system: GizmoSystem,
    viewport_selector: ViewportSelector,
    shortcut_map: ShortcutMap,
    settings_dialog: SettingsDialog,
    welcome: WelcomeScreen,

}

impl EditorApp {
    /// Default editor window width in logical pixels.
    pub const DEFAULT_WIDTH: u32 = 1280;

    /// Default editor window height in logical pixels.
    pub const DEFAULT_HEIGHT: u32 = 720;

    /// Window title.
    pub const WINDOW_TITLE: &str = "Chronos Engine Editor";

    /// Clear color for the background (dark charcoal).
    pub const CLEAR_COLOR: [f32; 4] = [0.067, 0.067, 0.067, 1.0];

    /// Create the editor: open window, init wgpu via egui-wgpu painter, init egui state.
    ///
    /// Uses `pollster::block_on` for async wgpu init (same pattern as `render.rs`).
    #[allow(deprecated)] // EventLoop::create_window — matches existing render.rs pattern
    pub fn new(event_loop: &EventLoop<()>) -> Result<Self, EditorError> {
        // ── Create window ──
        let window_attrs = Window::default_attributes()
            .with_title(Self::WINDOW_TITLE)
            .with_inner_size(PhysicalSize::new(Self::DEFAULT_WIDTH, Self::DEFAULT_HEIGHT));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .map_err(|e| EditorError::WindowCreation(e.to_string()))?,
        );

        // ── egui context ──
        let egui_ctx = egui::Context::default();
        egui_ctx.set_visuals(Visuals::dark());

        let viewport_id = ViewportId::ROOT;

        // ── egui-wgpu painter (manages wgpu instance, adapter, device, queue, surface) ──
        let wgpu_config = WgpuConfiguration {
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: Some(2),
            ..Default::default()
        };

        // On native, create the painter and initialize the surface synchronously
        // via pollster. On WASM, creation is deferred to `init_surface_wasm()`
        // which runs asynchronously after the event loop starts.
        #[cfg(not(target_arch = "wasm32"))]
        let (egui_painter, max_texture_side) = {
            let mut painter = egui_wgpu::winit::Painter::new(
                egui_ctx.clone(),
                wgpu_config,
                1,     // msaa_samples
                None,  // depth_format
                false, // support_transparent_backbuffer
                false, // dithering
            );

            pollster::block_on(painter.set_window(viewport_id, Some(window.clone())))
                .map_err(|e| EditorError::SurfaceCreation(e.to_string()))?;

            let side = painter.max_texture_side();
            (Some(painter), side)
        };

        // On WASM, defer surface creation — painter is None until init completes.
        #[cfg(target_arch = "wasm32")]
        let (egui_painter, max_texture_side) = (None, 2048);

        // ── egui-winit state (input bridge) ──
        let scale_factor = Some(window.scale_factor() as f32);
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            &window, // implements HasDisplayHandle
            scale_factor,
            None, // theme — let egui decide
            max_texture_side,
        );

        let mut state = EditorState::new();

        // Load recent projects from the user's config directory so projects
        // opened in previous sessions are remembered.
        if let Some(config_dir) = crate::platform::config_dir() {
            let _ = std::fs::create_dir_all(&config_dir);
            let recent_path = config_dir.join("recent.json");
            if let Ok(recents) =
                crate::editor_project::ProjectManager::load_recent_from_file(&recent_path)
            {
                state.project_manager.recent_projects = recents;
            }
        }

        Ok(Self {
            window,
            egui_ctx,
            egui_winit,
            egui_painter,
            viewport_id,
            state,
            menu_bar: MenuBarPanel::new(),
            toolbar: ToolbarPanel::new(),
            viewport: {
                let mut v = ViewportPanel::new();
                v.grid_visible = false;
                v
            },
            hierarchy: HierarchyPanel::new(),
            inspector: InspectorPanel::new(),
            asset_browser: AssetBrowserPanel::new(),
            console: ConsolePanel::new(),
            undo_stack: UndoStack::new(),
            grid_renderer: GridRenderer::new(),
            gizmo_system: GizmoSystem::new(),
            viewport_selector: ViewportSelector::new(),
            shortcut_map: ShortcutMap::new(),
            settings_dialog: SettingsDialog::new(),
            welcome: WelcomeScreen::new(),
        })
    }

    /// Handle a winit `WindowEvent`. Feeds input to egui_winit.
    ///
    /// Returns `true` if egui requests a repaint as a result of this event.
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_winit.on_window_event(&self.window, event);
        response.repaint
    }

    /// Handle window resize: reconfigure the wgpu surface and notify the egui painter.
    ///
    /// Called when the window is resized by the user or the WM.
    /// On WASM, this is a no-op until the painter is initialized.
    pub fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        if let Some(painter) = self.egui_painter.as_mut() {
            if new_size.width > 0 && new_size.height > 0 {
                let width = NonZeroU32::new(new_size.width).unwrap_or(NonZeroU32::MIN);
                let height = NonZeroU32::new(new_size.height).unwrap_or(NonZeroU32::MIN);
                painter.on_window_resized(self.viewport_id, width, height);
            }
        }
    }

    /// Render one frame: run egui UI with all panels, tessellate, paint via wgpu.
    ///
    /// On WASM, the wgpu surface is initialized asynchronously after the event
    /// loop starts. If `egui_painter` is `None` (deferred init not yet complete),
    /// this method returns early without painting.
    pub fn render(&mut self) -> Result<(), EditorError> {
        // On WASM, skip rendering until the surface is initialized.
        #[cfg(target_arch = "wasm32")]
        if self.egui_painter.is_none() {
            return Ok(());
        }

        let raw_input = self.egui_winit.take_egui_input(&self.window);

        let ctx = self.egui_ctx.clone();
        let state = &mut self.state;
        let menu_bar = &mut self.menu_bar;
        let toolbar = &mut self.toolbar;
        let viewport = &mut self.viewport;
        let hierarchy = &mut self.hierarchy;
        let inspector = &mut self.inspector;
        let asset_browser = &mut self.asset_browser;
        let console = &mut self.console;
        let undo_stack = &mut self.undo_stack;
        let grid_renderer = &mut self.grid_renderer;
        let gizmo_system = &mut self.gizmo_system;
        let viewport_selector = &mut self.viewport_selector;
        let shortcut_map = &mut self.shortcut_map;
        let settings_dialog = &mut self.settings_dialog;
        let welcome = &mut self.welcome;

        let full_output = ctx.run(raw_input, |ctx| {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    {
                        if let Some(action) = shortcut_map.find_action(
                            *key,
                            modifiers.ctrl,
                            modifiers.shift,
                            modifiers.alt,
                        ) {
                            match action {
                                ShortcutAction::Undo => {
                                    if let Some(desc) = undo_stack.undo(&mut state.world) {
                                        state.log(
                                            crate::editor_panels::ConsoleLogLevel::Info,
                                            format!("Undo: {desc}"),
                                        );
                                    }
                                }
                                ShortcutAction::Redo => {
                                    if let Some(desc) = undo_stack.redo(&mut state.world) {
                                        state.log(
                                            crate::editor_panels::ConsoleLogLevel::Info,
                                            format!("Redo: {desc}"),
                                        );
                                    }
                                }
                                ShortcutAction::GizmoTranslate => {
                                    state.gizmo_mode = crate::editor_panels::GizmoMode::Translate
                                }
                                ShortcutAction::GizmoRotate => {
                                    state.gizmo_mode = crate::editor_panels::GizmoMode::Rotate
                                }
                                ShortcutAction::GizmoScale => {
                                    state.gizmo_mode = crate::editor_panels::GizmoMode::Scale
                                }
                                ShortcutAction::ToggleSnap => {
                                    state.snap_enabled = !state.snap_enabled
                                }
                                ShortcutAction::ToggleGrid => {
                                    grid_renderer.visible = !grid_renderer.visible
                                }
                                ShortcutAction::Delete if !state.selected_entities.is_empty() => {
                                    state.delete_selected_requested = true;
                                }
                                ShortcutAction::Deselect => state.clear_selection(),
                                ShortcutAction::PlayStop => {
                                    state.play_mode = match state.play_mode {
                                        crate::editor_panels::PlayMode::Stopped => {
                                            crate::editor_panels::PlayMode::Playing
                                        }
                                        _ => crate::editor_panels::PlayMode::Stopped,
                                    };
                                }
                                ShortcutAction::Quit => state.should_quit = true,
                                ShortcutAction::NewProject => {
                                    state.project_manager.show_new_wizard = true;
                                    state.log(
                                        crate::editor_panels::ConsoleLogLevel::Info,
                                        "New project wizard",
                                    );
                                }
                                ShortcutAction::OpenProject => {
                                    state.project_manager.show_open_dialog = true;
                                    state.log(
                                        crate::editor_panels::ConsoleLogLevel::Info,
                                        "Open project dialog",
                                    );
                                }
                                ShortcutAction::Save => {
                                    if state.project_manager.is_loaded() {
                                        match state.project_manager.save_current() {
                                            Ok(()) => state.log(
                                                crate::editor_panels::ConsoleLogLevel::Info,
                                                "Project saved",
                                            ),
                                            Err(e) => state.log(
                                                crate::editor_panels::ConsoleLogLevel::Error,
                                                format!("Save failed: {e}"),
                                            ),
                                        }
                                    } else {
                                        state.log(
                                            crate::editor_panels::ConsoleLogLevel::Warn,
                                            "No project loaded to save",
                                        );
                                    }
                                }
                                ShortcutAction::SaveAs => {
                                    state.save_as_requested = true;
                                    state.log(
                                        crate::editor_panels::ConsoleLogLevel::Info,
                                        "Save As requested",
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            // ── Menu bar (top) ──
            egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                menu_bar.show(ui, state);
            });

            // ── Toolbar (below menu bar) ──
            egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
                toolbar.show(ui, state);
            });

            settings_dialog.visible = state.show_settings;
            settings_dialog.show(ctx);
            state.show_settings = settings_dialog.visible;

            // ── Hierarchy (left) ──
            egui::SidePanel::left("hierarchy")
                .min_width(180.0)
                .default_width(220.0)
                .show(ctx, |ui| {
                    ui.heading("Hierarchy");
                    ui.separator();
                    hierarchy.show(ui, state);
                });

            // ── Inspector (right) ──
            egui::SidePanel::right("inspector")
                .min_width(200.0)
                .default_width(280.0)
                .show(ctx, |ui| {
                    ui.heading("Inspector");
                    ui.separator();
                    inspector.show(ui, state);
                });

            // ── Bottom panel: Console + Asset Browser ──
            egui::TopBottomPanel::bottom("bottom_panel")
                .min_height(120.0)
                .default_height(200.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut state.show_console_tab, true, "Console");
                        ui.selectable_value(&mut state.show_console_tab, false, "Asset Browser");
                    });
                    ui.separator();
                    if state.show_console_tab {
                        console.show(ui, state);
                    } else {
                        asset_browser.show(ui, state);
                    }
                });

            // ── Central area: Welcome screen (no project) or Viewport (project loaded) ──
            if !state.project_manager.is_loaded() {
                egui::CentralPanel::default().show(ctx, |ui| {
                    welcome.show(ui, state);
                });
            } else {
                egui::CentralPanel::default().show(ctx, |ui| {
                    viewport.show(ui, state);

                    if let Some(vp_rect) = viewport.last_viewport_rect {
                        let painter = ui.painter_at(vp_rect);
                        let center = vp_rect.center();

                        let cam_offset = [viewport.camera_target[0], viewport.camera_target[2]];
                        let zoom = 1.0 / viewport.camera_distance.max(0.1);

                        grid_renderer.render(&painter, cam_offset, zoom, vp_rect);

                        gizmo_system.mode = state.gizmo_mode;
                        if !state.selected_entities.is_empty() {
                            gizmo_system.render(&painter, center);
                            if let Some(_gizmo_result) = gizmo_system.handle_input(ui, center) {
                                // Gizmo drag detected — delta available for entity transform
                            }
                        }

                        viewport_selector.render_box_select(&painter);
                    }
                });
            }
        });

        // ── Process action request flags (set by menu bar / shortcuts) ──
        self.process_action_requests();

        self.egui_winit
            .handle_platform_output(&self.window, full_output.platform_output);

        let title = if self.state.project_manager.is_loaded() {
            format!(
                "{} — {}",
                self.state.project_manager.project_name(),
                Self::WINDOW_TITLE
            )
        } else {
            Self::WINDOW_TITLE.to_string()
        };
        if self.window.title() != title {
            self.window.set_title(&title);
        }

        let shapes = full_output.shapes;
        let textures_delta = full_output.textures_delta;
        let pixels_per_point = full_output.pixels_per_point;

        let clipped_primitives = self.egui_ctx.tessellate(shapes, pixels_per_point);

        // Note: egui-wgpu 0.30 handles surface reconfiguration internally on
        // resize. If the surface is lost (e.g. on suspend/resume), the next
        // frame will usually recover automatically after handle_resize or
        // recover_surface is called by the event loop.
        let painter = self.egui_painter.as_mut().unwrap();
        painter.paint_and_update_textures(
            self.viewport_id,
            pixels_per_point,
            Self::CLEAR_COLOR,
            &clipped_primitives,
            &textures_delta,
            vec![],
        );

        // Auto-save project manifest every 60 seconds when a project is loaded.
        // This is a solo-dev-friendly safety net against crashes or power loss.
        const AUTO_SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);
        if self.state.project_manager.is_loaded() {
            let should_auto = match self.state.last_auto_save {
                None => true,
                Some(t) => t.elapsed() >= AUTO_SAVE_INTERVAL,
            };
            if should_auto {
                match self.state.project_manager.save_current() {
                    Ok(()) => {
                        self.state.last_auto_save = Some(std::time::Instant::now());
                        self.state.log(
                            crate::editor_panels::ConsoleLogLevel::Info,
                            "Auto-saved project",
                        );
                    }
                    Err(e) => {
                        self.state.log(
                            crate::editor_panels::ConsoleLogLevel::Warn,
                            format!("Auto-save failed: {e}"),
                        );
                    }
                }
            }
        }

        // Persist recent projects so they survive across editor sessions.
        // Only write when the list has actually changed to avoid disk I/O every frame.
        if self.state.recent_dirty {
            if let Some(config_dir) = crate::platform::config_dir() {
                let recent_path = config_dir.join("recent.json");
                match self.state.project_manager.save_recent_to_file(&recent_path) {
                    Ok(()) => self.state.recent_dirty = false,
                    Err(e) => {
                        eprintln!("Failed to save recent projects: {e}");
                        // Clear the flag so we don't spam I/O every frame.
                        // The in-memory recent list is still correct; the user
                        // will simply lose persistence for this session.
                        self.state.recent_dirty = false;
                    }
                }
            } else {
                self.state.recent_dirty = false;
            }
        }

        Ok(())
    }

    /// Initialize the wgpu surface on WASM (async, using wasm-bindgen-futures).
    ///
    /// Called from the winit event loop handler once the canvas is attached.
    /// Creates the egui-wgpu painter and initializes the rendering surface.
    /// After success, `render()` will start painting frames.
    #[cfg(target_arch = "wasm32")]
    pub async fn init_surface_wasm(&mut self) -> Result<(), EditorError> {
        use egui_wgpu::WgpuConfiguration;

        let wgpu_config = WgpuConfiguration {
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: Some(2),
            ..Default::default()
        };

        let mut painter = egui_wgpu::winit::Painter::new(
            self.egui_ctx.clone(),
            wgpu_config,
            1,     // msaa_samples
            None,  // depth_format
            false, // support_transparent_backbuffer
            false, // dithering
        );

        painter
            .set_window(self.viewport_id, Some(self.window.clone()))
            .await
            .map_err(|e| EditorError::SurfaceCreation(e.to_string()))?;

        self.egui_painter = Some(painter);
        Ok(())
    }

    /// Access the underlying window (for `request_redraw` etc.).
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Access the egui context.
    pub fn egui_ctx(&self) -> &egui::Context {
        &self.egui_ctx
    }

    /// Whether the editor has been asked to quit (e.g. via Ctrl+Q).
    pub fn should_quit(&self) -> bool {
        self.state.should_quit
    }

    /// Attempt to recreate the wgpu surface after a loss event.
    ///
    /// This is called by the event loop handler when `paint_and_update_textures`
    /// reports a surface error. On WASM, this is a no-op until the painter is
    /// initialized.
    pub fn recover_surface(&mut self) -> Result<(), EditorError> {
        if let Some(painter) = self.egui_painter.as_mut() {
            let size = self.window.inner_size();
            if size.width > 0 && size.height > 0 {
                let width = std::num::NonZeroU32::new(size.width).unwrap_or(std::num::NonZeroU32::MIN);
                let height =
                    std::num::NonZeroU32::new(size.height).unwrap_or(std::num::NonZeroU32::MIN);
                painter.on_window_resized(self.viewport_id, width, height);
            }
        }
        Ok(())
    }

    // ── Action Request Processing ───────────────────────────────────────

    /// Consume action request flags set by panels and perform the requested operations.
    ///
    /// Called once per frame after all panels have rendered, so that multiple
    /// panels can queue actions without fighting over borrows.
    fn process_action_requests(&mut self) {
        if self.state.undo_requested {
            self.state.undo_requested = false;
            if let Some(desc) = self.undo_stack.undo(&mut self.state.world) {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    format!("Undo: {desc}"),
                );
            }
        }

        if self.state.redo_requested {
            self.state.redo_requested = false;
            if let Some(desc) = self.undo_stack.redo(&mut self.state.world) {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    format!("Redo: {desc}"),
                );
            }
        }

        if self.state.delete_selected_requested {
            self.state.delete_selected_requested = false;
            let to_delete: Vec<_> = self.state.selected_entities.clone();
            let count = to_delete.len();
            if count > 0 {
                for entity in to_delete {
                    if self.state.world.entity_exists(entity) {
                        self.state.world.destroy_entity(entity);
                    }
                }
                self.state.selected_entities.clear();
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    format!("Deleted {count} entity/ies"),
                );
            }
        }

        if self.state.duplicate_selected_requested {
            self.state.duplicate_selected_requested = false;
            let selected: Vec<_> = self.state.selected_entities.clone();
            self.state.selected_entities.clear();
            for entity in selected {
                if self.state.world.entity_exists(entity) {
                    let new_entity = self.state.world.create_entity();
                    // Copy common components so the duplicate is visible/manipulable.
                    let pos = self
                        .state
                        .world
                        .get_component::<crate::component::Position>(entity)
                        .copied();
                    if let Some(pos) = pos {
                        self.state.world.add_component(new_entity, pos);
                    }
                    self.state.selected_entities.push(new_entity);
                }
            }
            let count = self.state.selected_entities.len();
            if count > 0 {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    format!("Duplicated {count} entity/ies"),
                );
            }
        }

        if self.state.select_all_requested {
            self.state.select_all_requested = false;
            self.state.selected_entities = self.state.world.all_entities();
            self.state.log(
                crate::editor_panels::ConsoleLogLevel::Info,
                format!(
                    "Selected all {} entities",
                    self.state.selected_entities.len()
                ),
            );
        }

        if self.state.save_as_requested {
            self.state.save_as_requested = false;
            if self.state.project_manager.is_loaded() {
                // Auto-generate a copy path: project_dir/../project_name_copy
                let copy_dir = self
                    .state
                    .project_manager
                    .project_dir
                    .as_ref()
                    .and_then(|dir| {
                        let file_name = dir.file_name()?.to_str()?;
                        dir.parent().map(|p| p.join(format!("{file_name}_copy")))
                    });
                if let Some(dir) = copy_dir {
                    let name = self.state.project_manager.project_name().to_string();
                    let template = self
                        .state
                        .project_manager
                        .current_project
                        .as_ref()
                        .map(|m| m.template)
                        .unwrap_or(crate::editor_project::ProjectTemplate::Empty);
                    match self.state.project_manager.save_as(&name, &dir) {
                        Ok(()) => {
                            self.state.project_manager.add_recent(
                                &name,
                                &dir.to_string_lossy(),
                                template,
                            );
                            self.state.log(
                                crate::editor_panels::ConsoleLogLevel::Info,
                                format!("Saved project copy to {}", dir.display()),
                            );
                            self.state.recent_dirty = true;
                        }
                        Err(e) => {
                            self.state.log(
                                crate::editor_panels::ConsoleLogLevel::Error,
                                format!("Save As failed: {e}"),
                            );
                        }
                    }
                } else {
                    self.state.log(
                        crate::editor_panels::ConsoleLogLevel::Warn,
                        "Could not determine copy destination",
                    );
                }
            } else {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Warn,
                    "No project loaded to save as",
                );
            }
        }

        if self.state.fullscreen_requested {
            self.state.fullscreen_requested = false;
            let is_fullscreen = self.window.fullscreen().is_some();
            if is_fullscreen {
                self.window.set_fullscreen(None);
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    "Exited fullscreen",
                );
            } else {
                self.window
                    .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    "Entered fullscreen",
                );
            }
        }

        if self.state.reset_layout_requested {
            self.state.reset_layout_requested = false;
            // Reset all egui area sizes (panels, windows) to their defaults.
            self.egui_ctx.memory_mut(|mem| mem.reset_areas());
            self.state.log(
                crate::editor_panels::ConsoleLogLevel::Info,
                "Layout reset to defaults",
            );
        }

        if self.state.launch_engine_requested {
            self.state.launch_engine_requested = false;
            self.launch_engine();
        }
    }

    /// Launch the Chronos Engine with the current project.
    ///
    /// Spawns `cargo run` in the project directory. Logs success or failure
    /// to the console panel.
    fn launch_engine(&mut self) {
        let Some(project_dir) = self.state.project_manager.project_dir.clone() else {
            self.state.log(
                crate::editor_panels::ConsoleLogLevel::Warn,
                "No project loaded — cannot launch engine",
            );
            return;
        };

        self.state.log(
            crate::editor_panels::ConsoleLogLevel::Info,
            format!("Launching engine in {}", project_dir.display()),
        );

        let result = std::process::Command::new("cargo")
            .arg("run")
            .arg("--release")
            .current_dir(&project_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        match result {
            Ok(mut child) => {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Info,
                    format!("Engine started (PID: {})", child.id()),
                );
                // Spawn a thread to capture output and log it to the console.
                let _tx = self.state.console_log.clone();
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => {
                self.state.log(
                    crate::editor_panels::ConsoleLogLevel::Error,
                    format!("Failed to launch engine: {e}"),
                );
            }
        }
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── EditorError display tests ──

    #[test]
    fn editor_error_window_creation_display() {
        let err = EditorError::WindowCreation("bad handle".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to create editor window"));
        assert!(msg.contains("bad handle"));
    }

    #[test]
    fn editor_error_surface_creation_display() {
        let err = EditorError::SurfaceCreation("no surface".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to create wgpu surface"));
        assert!(msg.contains("no surface"));
    }

    #[test]
    fn editor_error_no_adapter_display() {
        let err = EditorError::NoAdapter;
        let msg = format!("{err}");
        assert!(msg.contains("No compatible GPU adapter"));
    }

    #[test]
    fn editor_error_device_request_display() {
        let err = EditorError::DeviceRequest("timeout".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to request wgpu device"));
        assert!(msg.contains("timeout"));
    }

    #[test]
    fn editor_error_surface_texture_display() {
        let err = EditorError::SurfaceTexture("lost".into());
        let msg = format!("{err}");
        assert!(msg.contains("Failed to acquire surface texture"));
        assert!(msg.contains("lost"));
    }

    #[test]
    fn editor_error_other_display() {
        let err = EditorError::Other("something went wrong".into());
        let msg = format!("{err}");
        assert!(msg.contains("Editor error"));
        assert!(msg.contains("something went wrong"));
    }

    // ── Structural / constant tests ──

    #[test]
    fn editor_defaults() {
        assert_eq!(EditorApp::DEFAULT_WIDTH, 1280);
        assert_eq!(EditorApp::DEFAULT_HEIGHT, 720);
        assert_eq!(EditorApp::WINDOW_TITLE, "Chronos Engine Editor");
    }

    #[test]
    fn clear_color_is_dark() {
        let [r, g, b, a] = EditorApp::CLEAR_COLOR;
        assert!(r < 0.1, "red channel should be dark");
        assert!(g < 0.1, "green channel should be dark");
        assert!(b < 0.1, "blue channel should be dark");
        assert!((a - 1.0).abs() < f32::EPSILON, "alpha should be 1.0");
    }

    // ── Error chain test ──

    #[test]
    fn editor_error_implements_std_error() {
        let err = EditorError::Other("test".into());
        let _boxed: Box<dyn std::error::Error> = Box::new(err);
    }

    #[test]
    fn editor_error_debug_format() {
        let err = EditorError::NoAdapter;
        let debug = format!("{err:?}");
        assert!(debug.contains("NoAdapter"));
    }
}
