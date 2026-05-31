#[cfg(feature = "game")]
use std::any::Any;

use crate::plugin::editor::{
    DockZone, EditorPanel, EditorPluginHooks, InspectorHook, ToolbarButton,
};
use crate::plugin::{Plugin, PluginContext, PluginManifest};

use super::runner::{ChronosCompanyGame, GameConfig, GameMode};

// ── ChronosCompanyPlugin ───────────────────────────────────────────────────

/// A Phase-13 plugin that embeds the full Chronos Company RPG into the engine.
///
/// When registered with the `PluginRegistry`, this plugin:
/// - Initializes a `ChronosCompanyGame` on `on_init`
/// - Ticks the simulation every frame via `on_update`
/// - Exposes editor hooks for squad inspection, world map, and faction panels
/// - Logs game state changes through the plugin context
///
/// # Usage
///
/// ```rust,ignore
/// use chronos_engine::plugin::PluginRegistry;
/// use chronos_engine::game::plugin::ChronosCompanyPlugin;
///
/// let mut registry = PluginRegistry::new();
/// registry.register(Box::new(ChronosCompanyPlugin::new(GameConfig::default())));
/// ```
pub struct ChronosCompanyPlugin {
    game: Option<ChronosCompanyGame>,
    config: GameConfig,
    editor: ChronosCompanyEditorHooks,
    init_logged: bool,
}

impl ChronosCompanyPlugin {
    /// Create a new plugin instance with the given game configuration.
    pub fn new(config: GameConfig) -> Self {
        ChronosCompanyPlugin {
            game: None,
            config,
            editor: ChronosCompanyEditorHooks::new(),
            init_logged: false,
        }
    }

    /// Create with default configuration (Campaign mode, 4 mercenaries).
    pub fn default_campaign() -> Self {
        Self::new(GameConfig::default())
    }

    /// Create a sandbox mode plugin with a small map for quick testing.
    pub fn sandbox() -> Self {
        let config = GameConfig {
            mode: GameMode::Sandbox,
            map_width: 20,
            map_height: 20,
            start_squad_size: 2,
            start_gold: 200,
            ..GameConfig::default()
        };
        Self::new(config)
    }

    /// Access the inner game runner if initialized.
    pub fn game(&self) -> Option<&ChronosCompanyGame> {
        self.game.as_ref()
    }

    /// Mutable access to the inner game runner if initialized.
    pub fn game_mut(&mut self) -> Option<&mut ChronosCompanyGame> {
        self.game.as_mut()
    }

    /// Start a new game if the runner exists.
    pub fn start_new_game(&mut self) {
        if let Some(game) = &mut self.game {
            game.new_game();
        }
    }

    /// Advance the simulation by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        if let Some(game) = &mut self.game {
            game.tick(dt);
            game.check_squad_casualties();
        }
    }

    /// Return a status summary string for the editor status bar.
    pub fn status_summary(&self) -> String {
        self.game
            .as_ref()
            .map(|g| g.status_summary())
            .unwrap_or_else(|| "Chronos Company — not started".to_string())
    }
}

impl Plugin for ChronosCompanyPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest::new("com.chronos.rpg", "1.0.0")
            .with_display_name("Chronos Company")
            .with_author("Chronos Engine Team")
            .with_description(
                "A 3D real-time strategy open-world RPG sandbox. \
                 Command a band of mercenaries navigating an open world, \
                 taking job boards and bounties, moving as a unit, \
                 fighting in RTS-style tactical combat.",
            )
            .with_min_engine_version("1.0.0")
            .with_editor_hooks(true)
    }

    fn on_init(&mut self, ctx: &mut PluginContext) {
        let mut game = ChronosCompanyGame::new(self.config.clone());
        game.new_game();
        self.game = Some(game);
        self.init_logged = true;
        ctx.log_info(format!(
            "Chronos Company initialized — {} mode, {}×{} map, squad of {}",
            match self.config.mode {
                GameMode::Campaign => "Campaign",
                GameMode::Sandbox => "Sandbox",
                GameMode::Tutorial => "Tutorial",
            },
            self.config.map_width,
            self.config.map_height,
            self.config.start_squad_size,
        ));
    }

    fn on_update(&mut self, ctx: &mut PluginContext, dt: f32) {
        if let Some(game) = &mut self.game {
            let tick_before = game.state.tick;
            game.tick(dt as f64);
            game.check_squad_casualties();

            // Log significant events
            if game.state.tick > tick_before && game.state.tick % 600 == 0 {
                // Every ~10 seconds at 60fps
                ctx.log_info(game.status_summary());
            }

            // Log game over
            if !game.state.is_running && game.state.tick > 0 {
                ctx.log_warn("Game over — squad wiped out.");
            }
        }
    }

    fn on_shutdown(&mut self, ctx: &mut PluginContext) {
        if let Some(game) = &mut self.game {
            let play_time = game.state.total_play_time;
            ctx.log_info(format!(
                "Chronos Company shutting down. Play time: {:.1}s, encounters: {}, jobs: {}",
                play_time, game.state.encounters_completed, game.state.jobs_completed
            ));
        }
        self.game = None;
    }

    fn editor_hooks(&mut self) -> Option<&mut dyn EditorPluginHooks> {
        Some(&mut self.editor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

// ── ChronosCompanyEditorHooks ──────────────────────────────────────────────

/// Editor extension hooks for the Chronos Company plugin.
///
/// Provides custom panels:
/// - **Squad Inspector** (Right dock): Live squad health, stats, level
/// - **World Map** (Central dock): ASCII overview of explored terrain
/// - **Factions** (Right dock): Reputation standings with all factions
pub struct ChronosCompanyEditorHooks {
    pub squad_panel_open: bool,
    pub world_map_open: bool,
    pub faction_panel_open: bool,
    pub selected_squad_index: Option<usize>,
}

impl Default for ChronosCompanyEditorHooks {
    fn default() -> Self {
        Self::new()
    }
}

impl ChronosCompanyEditorHooks {
    pub fn new() -> Self {
        ChronosCompanyEditorHooks {
            squad_panel_open: true,
            world_map_open: false,
            faction_panel_open: true,
            selected_squad_index: None,
        }
    }
}

impl EditorPluginHooks for ChronosCompanyEditorHooks {
    fn panels(&mut self) -> Vec<EditorPanel> {
        vec![
            EditorPanel {
                id: "squad_inspector".into(),
                title: "Squad Inspector".into(),
                default_width: 280.0,
                default_height: 400.0,
                default_zone: DockZone::Right,
            },
            EditorPanel {
                id: "world_map".into(),
                title: "World Map".into(),
                default_width: 600.0,
                default_height: 400.0,
                default_zone: DockZone::Central,
            },
            EditorPanel {
                id: "factions".into(),
                title: "Factions".into(),
                default_width: 260.0,
                default_height: 300.0,
                default_zone: DockZone::Right,
            },
        ]
    }

    fn inspectors(&mut self) -> Vec<InspectorHook> {
        vec![
            InspectorHook {
                component_name: "MercenaryStats".into(),
                title: "Mercenary Stats".into(),
            },
            InspectorHook {
                component_name: "CombatState".into(),
                title: "Combat State".into(),
            },
            InspectorHook {
                component_name: "NavigationAgent".into(),
                title: "Navigation".into(),
            },
        ]
    }

    fn toolbar_buttons(&mut self) -> Vec<ToolbarButton> {
        vec![
            ToolbarButton {
                id: "new_game".into(),
                tooltip: "Start New Game".into(),
                shortcut: Some("Ctrl+N".into()),
            },
            ToolbarButton {
                id: "save_game".into(),
                tooltip: "Save Game".into(),
                shortcut: Some("Ctrl+S".into()),
            },
            ToolbarButton {
                id: "load_game".into(),
                tooltip: "Load Game".into(),
                shortcut: Some("Ctrl+L".into()),
            },
            ToolbarButton {
                id: "toggle_pause".into(),
                tooltip: "Pause / Resume".into(),
                shortcut: Some("Space".into()),
            },
        ]
    }

    fn render_panel(&mut self, panel_id: &str) -> String {
        match panel_id {
            "squad_inspector" => {
                let mut out = String::from("══ Squad Inspector ══\n");
                out.push_str("(Live data requires game reference — placeholder)\n");
                out.push_str("• Squad member 1: Warrior, Lv.1, HP 100/100\n");
                out.push_str("• Squad member 2: Archer, Lv.1, HP 80/80\n");
                out.push_str("• Squad member 3: Mage, Lv.1, HP 60/60\n");
                out
            }
            "world_map" => {
                let mut out = String::from("══ World Map ══\n");
                out.push_str("(Procedural terrain overview — placeholder)\n\n");
                for y in 0..8 {
                    for x in 0..16 {
                        let c = if x == 8 && y == 4 {
                            'P' // player
                        } else if (x + y) % 7 == 0 {
                            'T' // town
                        } else if (x * y) % 11 == 0 {
                            'D' // dungeon
                        } else {
                            match (x + y) % 4 {
                                0 => '.',
                                1 => '"',
                                2 => '^',
                                _ => '~',
                            }
                        };
                        out.push(c);
                    }
                    out.push('\n');
                }
                out
            }
            "factions" => {
                let mut out = String::from("══ Faction Reputation ══\n");
                out.push_str("Chronos Company    [====      ]  40\n");
                out.push_str("City Watch         [========  ]  80\n");
                out.push_str("Black Market       [==        ]  20\n");
                out.push_str("Merchant Guild     [======    ]  60\n");
                out.push_str("Rebels             [====      ]  40\n");
                out.push_str("Corporate Security [          ]  0\n");
                out
            }
            _ => format!("Unknown panel: {}", panel_id),
        }
    }

    fn on_toolbar_click(&mut self, button_id: &str) {
        match button_id {
            "new_game" => self.squad_panel_open = true,
            "save_game" => {}
            "load_game" => {}
            "toggle_pause" => {}
            _ => {}
        }
    }

    fn on_inspect(&mut self, component_name: &str, entity_id: u32) -> String {
        format!(
            "Inspecting {} on entity {}\n  (Custom inspector would show live values here)",
            component_name, entity_id
        )
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginRegistry;
    use crate::world::World;

    #[test]
    fn plugin_manifest() {
        let plugin = ChronosCompanyPlugin::sandbox();
        let mf = plugin.manifest();
        assert_eq!(mf.name, "com.chronos.rpg");
        assert_eq!(mf.version, "1.0.0");
        assert_eq!(mf.display_name, "Chronos Company");
        assert!(mf.has_editor_hooks);
        assert!(mf.description.contains("RPG"));
    }

    #[test]
    fn plugin_lifecycle_init() {
        let mut plugin = ChronosCompanyPlugin::sandbox();
        let mut world = World::new();
        let mut log = Vec::new();
        let api = crate::plugin::PluginApi::new(&mut world);
        let mut ctx = PluginContext {
            api,
            dt: 0.0,
            tick: 0,
            log_buffer: &mut log,
        };
        plugin.on_init(&mut ctx);
        assert!(plugin.game().is_some());
        assert!(plugin.init_logged);
        assert!(log
            .iter()
            .any(|l| l.contains("Chronos Company initialized")));
    }

    #[test]
    fn plugin_lifecycle_update() {
        let mut plugin = ChronosCompanyPlugin::sandbox();
        let mut world = World::new();
        let mut log = Vec::new();

        // Init
        {
            let api = crate::plugin::PluginApi::new(&mut world);
            let mut ctx = PluginContext {
                api,
                dt: 0.0,
                tick: 0,
                log_buffer: &mut log,
            };
            plugin.on_init(&mut ctx);
        }

        // Update
        {
            let api = crate::plugin::PluginApi::new(&mut world);
            let mut ctx = PluginContext {
                api,
                dt: 0.016,
                tick: 1,
                log_buffer: &mut log,
            };
            plugin.on_update(&mut ctx, 0.016);
        }

        assert!(plugin.game().is_some());
        let game = plugin.game().unwrap();
        assert!(game.state.tick > 0);
    }

    #[test]
    fn plugin_lifecycle_shutdown() {
        let mut plugin = ChronosCompanyPlugin::sandbox();
        let mut world = World::new();
        let mut log = Vec::new();

        // Init
        {
            let api = crate::plugin::PluginApi::new(&mut world);
            let mut ctx = PluginContext {
                api,
                dt: 0.0,
                tick: 0,
                log_buffer: &mut log,
            };
            plugin.on_init(&mut ctx);
        }

        // Shutdown
        {
            let api = crate::plugin::PluginApi::new(&mut world);
            let mut ctx = PluginContext {
                api,
                dt: 0.0,
                tick: 0,
                log_buffer: &mut log,
            };
            plugin.on_shutdown(&mut ctx);
        }

        assert!(plugin.game().is_none());
        assert!(log.iter().any(|l| l.contains("shutting down")));
    }

    #[test]
    fn plugin_status_summary() {
        let mut plugin = ChronosCompanyPlugin::sandbox();
        assert_eq!(plugin.status_summary(), "Chronos Company — not started");

        let mut world = World::new();
        let mut log = Vec::new();
        let api = crate::plugin::PluginApi::new(&mut world);
        let mut ctx = PluginContext {
            api,
            dt: 0.0,
            tick: 0,
            log_buffer: &mut log,
        };
        plugin.on_init(&mut ctx);
        let summary = plugin.status_summary();
        assert!(summary.contains("Day"));
        assert!(summary.contains("Gold"));
    }

    #[test]
    fn plugin_registry_integration() {
        let mut reg = PluginRegistry::new();
        let plugin = ChronosCompanyPlugin::sandbox();
        reg.register(Box::new(plugin)).unwrap();
        assert_eq!(reg.len(), 1);

        let mf = reg.manifest(0).unwrap();
        assert_eq!(mf.name, "com.chronos.rpg");

        let mut world = World::new();
        reg.init_all(&mut world);
        reg.update_all(&mut world, 0.016, 1);
        reg.shutdown_all(&mut world);

        let plugin = reg.plugin_mut(0).unwrap();
        let downcast = plugin.as_any_mut().downcast_mut::<ChronosCompanyPlugin>();
        assert!(downcast.is_some());
        let cc = downcast.unwrap();
        assert!(cc.game().is_none()); // shut down
    }

    #[test]
    fn editor_hooks_panels() {
        let mut hooks = ChronosCompanyEditorHooks::new();
        let panels = hooks.panels();
        assert_eq!(panels.len(), 3);
        assert!(panels.iter().any(|p| p.id == "squad_inspector"));
        assert!(panels.iter().any(|p| p.id == "world_map"));
        assert!(panels.iter().any(|p| p.id == "factions"));
    }

    #[test]
    fn editor_hooks_inspectors() {
        let mut hooks = ChronosCompanyEditorHooks::new();
        let inspectors = hooks.inspectors();
        assert_eq!(inspectors.len(), 3);
        assert!(inspectors
            .iter()
            .any(|i| i.component_name == "MercenaryStats"));
    }

    #[test]
    fn editor_hooks_toolbar() {
        let mut hooks = ChronosCompanyEditorHooks::new();
        let buttons = hooks.toolbar_buttons();
        assert_eq!(buttons.len(), 4);
        assert!(buttons.iter().any(|b| b.id == "new_game"));
        assert!(buttons.iter().any(|b| b.id == "toggle_pause"));
    }

    #[test]
    fn editor_hooks_render_panels() {
        let mut hooks = ChronosCompanyEditorHooks::new();
        let squad = hooks.render_panel("squad_inspector");
        assert!(squad.contains("Squad Inspector"));

        let map = hooks.render_panel("world_map");
        assert!(map.contains("World Map"));

        let factions = hooks.render_panel("factions");
        assert!(factions.contains("Faction Reputation"));
    }

    #[test]
    fn plugin_as_any_downcast() {
        let plugin = ChronosCompanyPlugin::sandbox();
        let any_ref = plugin.as_any();
        assert!(any_ref.downcast_ref::<ChronosCompanyPlugin>().is_some());
    }
}
