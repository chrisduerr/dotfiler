use std::io::{self, Read};
use std::{fs, env, path};
use toml::{self, value};
use std::process;

use error;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub dotfiles: Option<Vec<Dotfile>>,
    pub variables: Option<value::Table>,
    pub scripts: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct Dotfile {
    pub template: String,
    pub target: String,
}

pub fn load_config(config_path: &str) -> Result<Config, error::DotfilerError> {
    let config_path = resolve_path(config_path, None)?;
    let mut buffer = String::new();
    fs::File::open(config_path)?.read_to_string(&mut buffer)?;
    Ok(toml::from_str(&buffer)?)
}

// Rust can't deal with "~", "$HOME" or relative paths, this takes care of that
// Also remove / at end of path
pub fn resolve_path(path: &str, working_dir: Option<&str>) -> Result<String, io::Error> {
    let mut command = format!("realpath -ms {}", path);
    if working_dir.is_some() {
        command = format!("cd {} && {}", working_dir.unwrap(), command);
    }

    let output = process::Command::new("sh").arg("-c")
        .arg(&command)
        .output()?;

    if !output.stderr.is_empty() {
        let msg = format!("Unable to resolve path using '{}':\n{}",
                          command,
                          String::from_utf8_lossy(&output.stderr).trim());
        return Err(io::Error::new(io::ErrorKind::InvalidInput, msg));
    }

    let resolved_out = output.stdout;
    Ok(String::from_utf8_lossy(&resolved_out).trim().to_string())
}

pub fn get_templates_path(config_path: &str) -> Result<path::PathBuf, io::Error> {
    let config_path = resolve_path(config_path, None)?;
    Ok(path::Path::new(&config_path).parent().unwrap().join("templates"))
}

pub fn get_working_dir() -> Result<String, io::Error> {
    let mut app_dir = env::current_exe()?;
    app_dir.pop();
    Ok(app_dir.to_string_lossy().to_string())
}




// -------------
//     TESTS
// -------------

// Again, this requires the user to be undeadleech
#[test]
fn resolve_home_path() {
    assert_eq!(resolve_path("~/Programming", None).unwrap(),
               "/home/undeadleech/Programming");
    assert_eq!(resolve_path("$HOME/Programming", None).unwrap(),
               "/home/undeadleech/Programming");
}

// Finally something that doesn't rely on anything
#[test]
fn resolve_root_path() {
    assert_eq!(resolve_path("/root/test", None).unwrap(), "/root/test");
}

// Only checks last part of String to make it independent from compile path
#[test]
fn working_dir_ends_in_dotfiler_debug_deps_dir() {
    let working_dir = get_working_dir().unwrap();
    assert_eq!(working_dir.ends_with("dotfiler/target/debug/deps"), true);
}
