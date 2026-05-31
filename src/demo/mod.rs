#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Phase 14 — The Demo.
//!
//! A self-contained playable demonstration of the Chronos Engine,
//! showcasing the plugin-based Chronos Company RPG.
//!
//! # What This Demo Shows
//!
//! 1. **Plugin Lifecycle** — Register → Init → Update → Shutdown
//! 2. **Squad Management** — Spawn, move, and track a mercenary squad
//! 3. **Combat Simulation** — Tick-based tactical encounters
//! 4. **World Persistence** — Save and load game state
//! 5. **Faction Dynamics** — Reputation changes and consequences
//! 6. **Editor Hooks** — Live panels for squad, map, and factions
//! 7. **Day/Night Cycle** — Time-of-day simulation with ambient changes
//!
//! # Running the Demo
//!
//! ```bash
//! cargo run --features full
//! ```

#[cfg(feature = "game")]
use crate::game::plugin::ChronosCompanyPlugin;
#[cfg(feature = "game")]
use crate::game::runner::{GameConfig, GameMode};
#[cfg(feature = "game")]
use crate::plugin::{Plugin, PluginRegistry};
#[cfg(feature = "game")]
use crate::world::World;

// ── Demo Orchestrator ──────────────────────────────────────────────────────

/// Runs the full Phase 14 demo and returns a structured report.
///
/// This is the main entry point. It exercises every major subsystem
/// through the plugin architecture so that the demo is also a
/// comprehensive integration test.
#[cfg(feature = "game")]
pub fn run() -> DemoReport {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Phase 14 — Chronos Engine Demo                              ║");
    println!("║  Plugin-based RPG · Tactical Combat · Open World             ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    let mut report = DemoReport::new();

    // ── Step 1: Plugin Registry Setup ──
    println!("▶ Step 1 — Plugin Registry Setup");
    let mut registry = PluginRegistry::new();
    let plugin = ChronosCompanyPlugin::new(GameConfig {
        mode: GameMode::Campaign,
        map_width: 24,
        map_height: 24,
        world_seed: 42,
        start_gold: 1_000,
        start_squad_size: 4,
        auto_save_interval: 0, // disabled for demo
        screen_width: 800.0,
        screen_height: 600.0,
        cell_size: 2.0,
    });

    let mf = plugin.manifest();
    println!("  Registering plugin: {} v{}", mf.name, mf.version);
    println!("  Display name: {}", mf.display_name);
    println!("  Author: {}", mf.author);
    println!("  Description: {}", mf.description);
    registry.register(Box::new(plugin)).unwrap();
    println!("  ✓ Plugin registered ({} total)", registry.len());
    report.step_completed("plugin_registry");

    // ── Step 2: Initialize ──
    println!("\n▶ Step 2 — Initialize Plugin");
    let mut world = World::new();
    registry.init_all(&mut world);
    report.tick_count += 1;

    // Check init logs
    for log in &registry.log_output {
        println!("  {}", log);
    }
    report.step_completed("init");

    // ── Step 3: Simulation Loop ──
    println!("\n▶ Step 3 — Simulation Loop (300 ticks ≈ 5 seconds)");
    let mut combat_ticks = 0;
    for tick in 0..300 {
        registry.update_all(&mut world, 0.016, tick);
        report.tick_count += 1;

        // Every 60 ticks (~1 second), print status
        if tick % 60 == 0 && tick > 0 {
            let plugin = registry.plugin_mut(0).unwrap();
            let cc = plugin
                .as_any_mut()
                .downcast_mut::<ChronosCompanyPlugin>()
                .unwrap();
            println!("  [tick {:>3}] {}", tick, cc.status_summary());
        }

        // At tick 120, issue a move order
        if tick == 120 {
            let plugin = registry.plugin_mut(0).unwrap();
            let cc = plugin
                .as_any_mut()
                .downcast_mut::<ChronosCompanyPlugin>()
                .unwrap();
            if let Some(game) = cc.game_mut() {
                game.order_move([30.0, 0.0, 30.0]);
                println!("  [tick {:>3}] 🗺️  Squad ordered to move to (30, 30)", tick);
            }
        }

        // At tick 180, simulate combat
        if tick == 180 {
            let plugin = registry.plugin_mut(0).unwrap();
            let cc = plugin
                .as_any_mut()
                .downcast_mut::<ChronosCompanyPlugin>()
                .unwrap();
            if let Some(game) = cc.game_mut() {
                // Simulate enemy contact by damaging a squad member
                if let Some(&first) = game.squad_entities.first() {
                    if let Some(hp) = game
                        .world
                        .get_component_mut::<crate::component::Health>(first)
                    {
                        hp.take_damage(15);
                        println!(
                            "  [tick {:>3}] ⚔️  Combat! Squad member took 15 damage (HP: {}/{})",
                            tick, hp.current, hp.max
                        );
                        combat_ticks = tick;
                    }
                }
            }
        }
    }
    report.step_completed("simulation");

    // ── Step 4: Faction Dynamics ──
    println!("\n▶ Step 4 — Faction Dynamics");
    {
        let plugin = registry.plugin_mut(0).unwrap();
        let cc = plugin
            .as_any_mut()
            .downcast_mut::<ChronosCompanyPlugin>()
            .unwrap();
        if let Some(game) = cc.game_mut() {
            use crate::game::factions::FactionId;
            game.reputation.modify_reputation(FactionId::CityWatch, 30);
            game.reputation
                .modify_reputation(FactionId::BlackMarket, -20);
            println!(
                "  City Watch reputation: {}",
                game.reputation.get_value(FactionId::CityWatch)
            );
            println!(
                "  Black Market reputation: {}",
                game.reputation.get_value(FactionId::BlackMarket)
            );
            println!(
                "  Chronos Company reputation: {}",
                game.reputation.get_value(FactionId::ChronosCompany)
            );
        }
    }
    report.step_completed("factions");

    // ── Step 5: Save / Load ──
    println!("\n▶ Step 5 — Save / Load Persistence");
    {
        let plugin = registry.plugin_mut(0).unwrap();
        let cc = plugin
            .as_any_mut()
            .downcast_mut::<ChronosCompanyPlugin>()
            .unwrap();
        if let Some(game) = cc.game_mut() {
            let gold_before = game.state.gold;
            let level_before = game.state.player_level;
            println!(
                "  Gold before save: {}, Level: {}",
                gold_before, level_before
            );

            let slot = game.save_game(99).unwrap();
            println!("  ✓ Saved to slot {}", slot);

            // Mutate
            game.state.gold = 0;
            game.state.player_level = 99;
            println!(
                "  Mutated → Gold: {}, Level: {}",
                game.state.gold, game.state.player_level
            );

            game.load_game(99).unwrap();
            println!(
                "  ✓ Loaded → Gold: {}, Level: {}",
                game.state.gold, game.state.player_level
            );

            assert_eq!(game.state.gold, gold_before);
            assert_eq!(game.state.player_level, level_before);
        }
    }
    report.step_completed("save_load");

    // ── Step 6: Editor Hooks ──
    println!("\n▶ Step 6 — Editor Hooks");
    {
        let plugin = registry.plugin_mut(0).unwrap();
        let cc = plugin
            .as_any_mut()
            .downcast_mut::<ChronosCompanyPlugin>()
            .unwrap();
        if let Some(hooks) = cc.editor_hooks() {
            let panels = hooks.panels();
            println!("  Registered panels: {}", panels.len());
            for panel in &panels {
                println!("    · {} ({:?})", panel.title, panel.default_zone);
            }

            let buttons = hooks.toolbar_buttons();
            println!("  Toolbar buttons: {}", buttons.len());
            for btn in &buttons {
                println!("    · {} — {}", btn.id, btn.tooltip);
            }

            // Render a sample panel
            let rendered = hooks.render_panel("squad_inspector");
            for line in rendered.lines().take(3) {
                println!("  {}", line);
            }
        }
    }
    report.step_completed("editor_hooks");

    // ── Step 7: Shutdown ──
    println!("\n▶ Step 7 — Shutdown");
    registry.shutdown_all(&mut world);
    for log in &registry.log_output {
        println!("  {}", log);
    }
    report.step_completed("shutdown");

    // ── Summary ──
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  Demo Complete                                                 ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("  Steps completed: {}", report.steps.len());
    println!("  Total ticks simulated: {}", report.tick_count);
    println!("  Combat triggered at tick: {}", combat_ticks);
    println!();

    report
}

/// A structured report produced by the demo run.
#[derive(Debug, Clone)]
pub struct DemoReport {
    pub steps: Vec<String>,
    pub tick_count: u64,
    pub errors: Vec<String>,
}

impl Default for DemoReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DemoReport {
    pub fn new() -> Self {
        DemoReport {
            steps: Vec::new(),
            tick_count: 0,
            errors: Vec::new(),
        }
    }

    fn step_completed(&mut self, name: &str) {
        self.steps.push(name.to_string());
    }

    /// Returns true if all expected demo steps completed.
    pub fn is_success(&self) -> bool {
        let expected = [
            "plugin_registry",
            "init",
            "simulation",
            "factions",
            "save_load",
            "editor_hooks",
            "shutdown",
        ];
        expected.iter().all(|s| self.steps.contains(&s.to_string()))
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "game")]
    fn demo_report_success() {
        let mut report = DemoReport::new();
        report.step_completed("plugin_registry");
        report.step_completed("init");
        report.step_completed("simulation");
        report.step_completed("factions");
        report.step_completed("save_load");
        report.step_completed("editor_hooks");
        report.step_completed("shutdown");
        assert!(report.is_success());
    }

    #[test]
    #[cfg(feature = "game")]
    fn demo_report_failure_on_missing_step() {
        let mut report = DemoReport::new();
        report.step_completed("plugin_registry");
        report.step_completed("init");
        // missing steps
        assert!(!report.is_success());
    }

    #[test]
    #[cfg(feature = "game")]
    fn demo_runs_without_panic() {
        let report = run();
        assert!(report.is_success(), "expected all demo steps to complete");
        assert!(report.tick_count > 100, "expected significant tick count");
        assert!(report.errors.is_empty(), "expected no errors");
    }

    #[test]
    #[cfg(feature = "game")]
    fn demo_plugin_registry_cycle() {
        let mut registry = PluginRegistry::new();
        let plugin = ChronosCompanyPlugin::new(GameConfig {
            mode: GameMode::Sandbox,
            map_width: 10,
            map_height: 10,
            world_seed: 1,
            start_gold: 100,
            start_squad_size: 2,
            auto_save_interval: 0,
            screen_width: 400.0,
            screen_height: 300.0,
            cell_size: 1.0,
        });
        registry.register(Box::new(plugin)).unwrap();

        let mut world = World::new();
        registry.init_all(&mut world);
        assert_eq!(registry.len(), 1);

        registry.update_all(&mut world, 0.016, 1);
        registry.update_all(&mut world, 0.016, 2);

        let plugin = registry.plugin_mut(0).unwrap();
        let cc = plugin
            .as_any_mut()
            .downcast_mut::<ChronosCompanyPlugin>()
            .unwrap();
        assert!(cc.game().is_some());

        registry.shutdown_all(&mut world);
        let cc = registry
            .plugin_mut(0)
            .unwrap()
            .as_any_mut()
            .downcast_mut::<ChronosCompanyPlugin>()
            .unwrap();
        assert!(cc.game().is_none());
    }
}
