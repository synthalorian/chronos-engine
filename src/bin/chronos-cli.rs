//! Chronos CLI — command-line interface for the Chronos Engine.
//!
//! Provides project scaffolding, building, running, testing, and packaging
//! commands for games built with Chronos.
//!
//! # Usage
//!
//! ```bash
//! chronos new my_game
//! chronos build --release
//! chronos run
//! chronos test
//! chronos package
//! ```

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

// ──────────────────────────────────────────────────────────────
// CLI Error
// ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum CliError {
    Io(std::io::Error),
    ProjectNotFound,
    InvalidProjectName(String),
    CargoFailed { cmd: String, status: i32 },
    TemplateNotFound(String),
    ParseError(String),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::Io(e) => write!(f, "IO error: {}", e),
            CliError::ProjectNotFound => write!(f, "no Chronos project found in current directory"),
            CliError::InvalidProjectName(name) => write!(f, "invalid project name: {}", name),
            CliError::CargoFailed { cmd, status } => {
                write!(f, "cargo command failed: {} (exit code: {})", cmd, status)
            }
            CliError::TemplateNotFound(t) => write!(f, "template not found: {}", t),
            CliError::ParseError(msg) => write!(f, "parse error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self {
        CliError::Io(e)
    }
}

// ──────────────────────────────────────────────────────────────
// Templates
// ──────────────────────────────────────────────────────────────

/// Available project templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    Empty,
    Platformer2D,
    Shooter3D,
    Rpg,
}

impl std::str::FromStr for ProjectTemplate {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "empty" | "default" => Ok(ProjectTemplate::Empty),
            "platformer" | "platformer2d" | "2d" => Ok(ProjectTemplate::Platformer2D),
            "shooter" | "shooter3d" | "3d" => Ok(ProjectTemplate::Shooter3D),
            "rpg" => Ok(ProjectTemplate::Rpg),
            _ => Err(CliError::TemplateNotFound(s.to_string())),
        }
    }
}

impl ProjectTemplate {
    pub fn template_name(&self) -> &'static str {
        match self {
            ProjectTemplate::Empty => "empty",
            ProjectTemplate::Platformer2D => "platformer2d",
            ProjectTemplate::Shooter3D => "shooter3d",
            ProjectTemplate::Rpg => "rpg",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProjectTemplate::Empty => "Empty Project",
            ProjectTemplate::Platformer2D => "2D Platformer",
            ProjectTemplate::Shooter3D => "3D Shooter",
            ProjectTemplate::Rpg => "RPG",
        }
    }
}

// ──────────────────────────────────────────────────────────────
// CLI Commands
// ──────────────────────────────────────────────────────────────

/// Parsed CLI arguments.
#[derive(Debug, Default)]
pub struct CliArgs {
    pub command: String,
    pub project_name: Option<String>,
    pub template: Option<ProjectTemplate>,
    pub release: bool,
    pub features: Vec<String>,
    pub verbose: bool,
    pub target_dir: Option<PathBuf>,
}

impl CliArgs {
    /// Parse arguments from the environment.
    pub fn from_env() -> Result<Self, CliError> {
        let args: Vec<String> = env::args().collect();
        Self::parse(&args)
    }

    /// Parse a raw argument vector.
    pub fn parse(args: &[String]) -> Result<Self, CliError> {
        if args.len() < 2 {
            return Err(CliError::ParseError("no command provided".into()));
        }

        let mut parsed = CliArgs {
            command: args[1].clone(),
            ..Default::default()
        };

        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--release" | "-r" => parsed.release = true,
                "--template" | "-t" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(CliError::ParseError("missing template value".into()));
                    }
                    match ProjectTemplate::from_str(&args[i]) {
                        Ok(t) => parsed.template = Some(t),
                        Err(_) => return Err(CliError::TemplateNotFound(args[i].clone())),
                    }
                }
                "--features" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(CliError::ParseError("missing features value".into()));
                    }
                    parsed.features = args[i].split(',').map(|s| s.trim().to_string()).collect();
                }
                "--verbose" | "-v" => parsed.verbose = true,
                "--target-dir" => {
                    i += 1;
                    if i >= args.len() {
                        return Err(CliError::ParseError("missing target-dir value".into()));
                    }
                    parsed.target_dir = Some(PathBuf::from(&args[i]));
                }
                other => {
                    if parsed.project_name.is_none() && !other.starts_with('-') {
                        parsed.project_name = Some(other.to_string());
                    }
                }
            }
            i += 1;
        }

        Ok(parsed)
    }
}

// ──────────────────────────────────────────────────────────────
// Project Creation
// ──────────────────────────────────────────────────────────────

/// Create a new Chronos project from a template.
pub fn new_project(name: &str, template: ProjectTemplate) -> Result<(), CliError> {
    validate_project_name(name)?;

    let project_dir = PathBuf::from(name);
    if project_dir.exists() {
        return Err(CliError::InvalidProjectName(format!(
            "directory '{}' already exists",
            name
        )));
    }

    fs::create_dir_all(&project_dir)?;
    fs::create_dir_all(project_dir.join("assets"))?;
    fs::create_dir_all(project_dir.join("assets").join("images"))?;
    fs::create_dir_all(project_dir.join("assets").join("audio"))?;
    fs::create_dir_all(project_dir.join("assets").join("fonts"))?;
    fs::create_dir_all(project_dir.join("assets").join("models"))?;
    fs::create_dir_all(project_dir.join("assets").join("scripts"))?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("scenes"))?;

    // Write Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
chronos-engine = {{ version = "1.0", features = ["full"] }}
"#,
        name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;

    // Write main.rs
    let main_rs = generate_main_rs(template);
    fs::write(project_dir.join("src").join("main.rs"), main_rs)?;

    // Write README
    let readme = format!(
        "# {}\n\nA Chronos Engine game.\n\n## Build\n\n```bash\nchronos build\n```\n\n## Run\n\n```bash\nchronos run\n```\n",
        name
    );
    fs::write(project_dir.join("README.md"), readme)?;

    // Write .gitignore
    fs::write(
        project_dir.join(".gitignore"),
        "/target\n*.lock\nassets/.cache\nassets/.thumbnails\n*.meta\n",
    )?;

    // Write a default scene
    let default_scene = r#"{
  "name": "main",
  "entities": []
}"#;
    fs::write(project_dir.join("scenes").join("main.scene"), default_scene)?;

    println!(
        "✓ Created '{}' ({}) in {}",
        name,
        template.display_name(),
        project_dir.canonicalize().unwrap_or(project_dir).display()
    );

    Ok(())
}

fn generate_main_rs(template: ProjectTemplate) -> String {
    match template {
        ProjectTemplate::Empty => r#"use chronos_engine::prelude::*;

fn main() {
    println!("Hello, Chronos!");
}
"#
        .to_string(),

        ProjectTemplate::Platformer2D => r#"use chronos_engine::prelude::*;

fn main() {
    let mut world = World::new();
    let mut game_loop = GameLoop::new(60.0);

    // Create player
    let player = world.create_entity();
    world.add_component(player, Position { x: 0.0, y: 0.0 });
    world.add_component(player, Velocity { x: 0.0, y: 0.0 });
    world.add_component(player, Sprite { atlas: "player".into(), frame: 0 });
    world.add_component(player, Grounded(false));
    world.add_component(player, Gravity(9.8));

    game_loop.run(&mut world);
}
"#
        .to_string(),

        ProjectTemplate::Shooter3D => r#"use chronos_engine::prelude::*;

fn main() {
    let mut world = World::new();
    let mut game_loop = GameLoop::new(60.0);

    // Create camera
    let camera = world.create_entity();
    world.add_component(camera, Position { x: 0.0, y: 5.0, z: -10.0 });

    // Create player
    let player = world.create_entity();
    world.add_component(player, Position { x: 0.0, y: 0.0, z: 0.0 });
    world.add_component(player, Velocity { x: 0.0, y: 0.0, z: 0.0 });

    game_loop.run(&mut world);
}
"#
        .to_string(),

        ProjectTemplate::Rpg => r#"use chronos_engine::prelude::*;

fn main() {
    let mut world = World::new();
    let mut game_loop = GameLoop::new(60.0);

    // Spawn a hero
    let hero = world.create_entity();
    world.add_component(hero, Position { x: 0.0, y: 0.0 });
    world.add_component(hero, Health { current: 100, max: 100 });
    world.add_component(hero, Sprite { atlas: "hero".into(), frame: 0 });

    game_loop.run(&mut world);
}
"#
        .to_string(),
    }
}

fn validate_project_name(name: &str) -> Result<(), CliError> {
    if name.is_empty() {
        return Err(CliError::InvalidProjectName("empty name".into()));
    }
    if name.contains(' ') || name.contains('\t') {
        return Err(CliError::InvalidProjectName(
            "name cannot contain whitespace".into(),
        ));
    }
    if !name.chars().next().unwrap_or('_').is_alphabetic() {
        return Err(CliError::InvalidProjectName(
            "name must start with a letter".into(),
        ));
    }
    if name
        .chars()
        .any(|c| !c.is_alphanumeric() && c != '_' && c != '-')
    {
        return Err(CliError::InvalidProjectName(
            "name can only contain letters, digits, underscores, and hyphens".into(),
        ));
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Build / Run / Test
// ──────────────────────────────────────────────────────────────

/// Run `cargo build` in the current project.
pub fn build_project(release: bool, features: &[String], verbose: bool) -> Result<(), CliError> {
    ensure_project()?;

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    if !features.is_empty() {
        cmd.arg("--features");
        cmd.arg(features.join(","));
    }
    if verbose {
        cmd.arg("--verbose");
    }

    run_cargo(cmd, "cargo build")
}

/// Run `cargo run` in the current project.
pub fn run_project(release: bool, features: &[String], verbose: bool) -> Result<(), CliError> {
    ensure_project()?;

    let mut cmd = Command::new("cargo");
    cmd.arg("run");
    if release {
        cmd.arg("--release");
    }
    if !features.is_empty() {
        cmd.arg("--features");
        cmd.arg(features.join(","));
    }
    if verbose {
        cmd.arg("--verbose");
    }

    run_cargo(cmd, "cargo run")
}

/// Run `cargo test` in the current project.
pub fn test_project(features: &[String], verbose: bool) -> Result<(), CliError> {
    ensure_project()?;

    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    if !features.is_empty() {
        cmd.arg("--features");
        cmd.arg(features.join(","));
    }
    if verbose {
        cmd.arg("--verbose");
    }

    run_cargo(cmd, "cargo test")
}

/// Package the project for distribution.
pub fn package_project(release: bool) -> Result<(), CliError> {
    ensure_project()?;

    if !release {
        println!("Building in release mode for packaging...");
        build_project(true, &[], false)?;
    }

    let name = project_name()?;
    let dist_dir = PathBuf::from(format!("{}-dist", name));
    let _ = fs::remove_dir_all(&dist_dir);
    fs::create_dir_all(&dist_dir)?;
    fs::create_dir_all(dist_dir.join("assets"))?;

    // Copy executable
    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.clone()
    };

    let exe_src = if release {
        PathBuf::from("target/release").join(&exe_name)
    } else {
        PathBuf::from("target/debug").join(&exe_name)
    };

    if exe_src.exists() {
        fs::copy(&exe_src, dist_dir.join(&exe_name))?;
    } else {
        return Err(CliError::CargoFailed {
            cmd: "build".into(),
            status: 1,
        });
    }

    // Copy assets
    copy_dir_all(Path::new("assets"), &dist_dir.join("assets"))?;

    println!(
        "✓ Packaged '{}' to {}",
        name,
        dist_dir.canonicalize().unwrap_or(dist_dir).display()
    );
    Ok(())
}

fn run_cargo(mut cmd: Command, description: &str) -> Result<(), CliError> {
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd.status()?;
    if !status.success() {
        return Err(CliError::CargoFailed {
            cmd: description.into(),
            status: status.code().unwrap_or(-1),
        });
    }
    Ok(())
}

fn ensure_project() -> Result<(), CliError> {
    if !Path::new("Cargo.toml").exists() {
        return Err(CliError::ProjectNotFound);
    }
    Ok(())
}
fn project_name() -> Result<String, CliError> {
    let cargo = fs::read_to_string("Cargo.toml")?;
    let mut in_package = false;
    for line in cargo.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = false;
            continue;
        }
        if in_package && trimmed.starts_with("name") {
            if let Some(quote_start) = trimmed.find('"') {
                if let Some(quote_end) = trimmed[quote_start + 1..].find('"') {
                    return Ok(trimmed[quote_start + 1..quote_start + 1 + quote_end].to_string());
                }
            }
        }
    }
    Ok("unknown".into())
}

fn copy_dir_all(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Help
// ──────────────────────────────────────────────────────────────

fn print_help() {
    println!("chronos — Chronos Engine CLI");
    println!();
    println!("USAGE:");
    println!("    chronos <command> [options]");
    println!();
    println!("COMMANDS:");
    println!("    new <name>        Create a new project");
    println!("    build             Build the current project");
    println!("    run               Build and run the current project");
    println!("    test              Run tests");
    println!("    package           Package for distribution");
    println!("    help              Show this message");
    println!();
    println!("OPTIONS:");
    println!("    -r, --release     Build in release mode");
    println!("    -t, --template    Project template (empty, platformer2d, shooter3d, rpg)");
    println!("    --features        Comma-separated feature flags");
    println!("    -v, --verbose     Verbose output");
    println!();
    println!("EXAMPLES:");
    println!("    chronos new my_game --template platformer2d");
    println!("    chronos build --release");
    println!("    chronos run --features \"render audio\"");
}

// ──────────────────────────────────────────────────────────────
// Main
// ──────────────────────────────────────────────────────────────

fn main() {
    let args = match CliArgs::from_env() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            print_help();
            std::process::exit(1);
        }
    };

    let result = match args.command.as_str() {
        "new" => {
            let name = args.project_name.as_deref().unwrap_or("my_game");
            let template = args.template.unwrap_or(ProjectTemplate::Empty);
            new_project(name, template)
        }
        "build" => build_project(args.release, &args.features, args.verbose),
        "run" => run_project(args.release, &args.features, args.verbose),
        "test" => test_project(&args.features, args.verbose),
        "package" => package_project(args.release),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        _ => {
            eprintln!("Unknown command: {}", args.command);
            print_help();
            std::process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_dir() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(format!("/tmp/chronos_cli_tests_{}", id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    /// Lock to serialize tests that mutate the process current working directory.
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // Test 1: parse help command.
    #[test]
    fn parse_help() {
        let args = CliArgs::parse(&["chronos".into(), "help".into()]).unwrap();
        assert_eq!(args.command, "help");
    }

    // Test 2: parse new with template.
    #[test]
    fn parse_new_with_template() {
        let args = CliArgs::parse(&[
            "chronos".into(),
            "new".into(),
            "my_game".into(),
            "--template".into(),
            "rpg".into(),
        ])
        .unwrap();
        assert_eq!(args.command, "new");
        assert_eq!(args.project_name, Some("my_game".into()));
        assert_eq!(args.template, Some(ProjectTemplate::Rpg));
    }

    // Test 3: parse build --release.
    #[test]
    fn parse_build_release() {
        let args = CliArgs::parse(&[
            "chronos".into(),
            "build".into(),
            "--release".into(),
            "--verbose".into(),
        ])
        .unwrap();
        assert_eq!(args.command, "build");
        assert!(args.release);
        assert!(args.verbose);
    }

    // Test 4: parse features.
    #[test]
    fn parse_features() {
        let args = CliArgs::parse(&[
            "chronos".into(),
            "run".into(),
            "--features".into(),
            "render,audio".into(),
        ])
        .unwrap();
        assert_eq!(args.features, vec!["render", "audio"]);
    }

    // Test 5: validate_project_name rejects bad names.
    #[test]
    fn validate_name_rejects_bad() {
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("hello world").is_err());
        assert!(validate_project_name("123abc").is_err());
        assert!(validate_project_name("a!b").is_err());
    }

    // Test 6: validate_project_name accepts good names.
    #[test]
    fn validate_name_accepts_good() {
        assert!(validate_project_name("my_game").is_ok());
        assert!(validate_project_name("game-2").is_ok());
        assert!(validate_project_name("A").is_ok());
    }

    // Test 7: new_project creates directory structure.
    #[test]
    fn new_project_creates_structure() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = test_dir();
        let original = env::current_dir().unwrap();
        env::set_current_dir(&dir).unwrap();

        new_project("test_proj", ProjectTemplate::Empty).expect("new");

        assert!(dir.join("test_proj").exists());
        assert!(dir.join("test_proj/src/main.rs").exists());
        assert!(dir.join("test_proj/Cargo.toml").exists());
        assert!(dir.join("test_proj/assets").exists());
        assert!(dir.join("test_proj/scenes/main.scene").exists());
        assert!(dir.join("test_proj/.gitignore").exists());

        env::set_current_dir(original).unwrap();
    }

    // Test 8: project_name parses Cargo.toml.
    #[test]
    fn project_name_parsing() {
        let _guard = CWD_LOCK.lock().unwrap();
        let dir = test_dir();
        let original = env::current_dir().unwrap();
        env::set_current_dir(&dir).unwrap();

        fs::write(
            "Cargo.toml",
            r#"[package]
name = "my-cool-game"
version = "0.1.0"
"#,
        )
        .unwrap();

        let name = project_name().unwrap();
        assert_eq!(name, "my-cool-game");

        env::set_current_dir(original).unwrap();
    }

    // Test 9: Template from_str.
    #[test]
    fn template_from_str() {
        assert_eq!(
            ProjectTemplate::from_str("empty").unwrap(),
            ProjectTemplate::Empty
        );
        assert_eq!(
            ProjectTemplate::from_str("2d").unwrap(),
            ProjectTemplate::Platformer2D
        );
        assert_eq!(
            ProjectTemplate::from_str("3d").unwrap(),
            ProjectTemplate::Shooter3D
        );
        assert_eq!(ProjectTemplate::from_str("rpg").unwrap(), ProjectTemplate::Rpg);
        assert!(ProjectTemplate::from_str("unknown").is_err());
    }

    // Test 10: Error display.
    #[test]
    fn error_display_messages() {
        let e = CliError::ProjectNotFound;
        assert!(e.to_string().contains("no Chronos project"));

        let e = CliError::InvalidProjectName("bad".into());
        assert!(e.to_string().contains("bad"));

        let e = CliError::CargoFailed {
            cmd: "build".into(),
            status: 101,
        };
        assert!(e.to_string().contains("101"));
    }
}
