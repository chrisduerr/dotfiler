use std::io::{self, Read, Write};
use std::{fs, env, path};
use toml::{self, value};
use std::process;
use handlebars;

use error;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub dotfiles: Vec<Dotfile>,
    pub variables: value::Table,
}

#[derive(Serialize, Deserialize)]
pub struct Dotfile {
    pub template: String,
    pub target: String,
}

pub fn load_config(config_path: &str) -> Result<Config, error::DotfilerError> {
    let config_path = resolve_path(config_path)?;
    let mut buffer = String::new();
    fs::File::open(config_path)?.read_to_string(&mut buffer)?;
    Ok(toml::from_str(&buffer)?)
}

// Rust can't deal with "~", "$HOME" or relative paths, this takes care of that
// Also remove / at end of path
pub fn resolve_path(path: &str) -> Result<String, error::DotfilerError> {
    let command = format!("realpath -ms {}", path);
    let output = process::Command::new("sh").arg("-c")
        .arg(&command)
        .output()?;
    let resolved_out = output.stdout;
    Ok(String::from_utf8_lossy(&resolved_out).trim().to_string())
}

pub fn get_home_dir() -> Result<String, io::Error> {
    let home_dir =
        env::home_dir().ok_or_else(|| {
                            io::Error::new(io::ErrorKind::NotFound,
                                           "Unable to locate home directory.")
                        })?;
    Ok(home_dir.to_string_lossy().to_string())
}

pub fn get_templates_path(config_path: &str) -> Result<path::PathBuf, io::Error> {
    Ok(path::Path::new(config_path)
           .parent()
           .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
           .join("templates"))
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
    assert_eq!(resolve_path("~/Programming").unwrap(),
               "/home/undeadleech/Programming");
    assert_eq!(resolve_path("$HOME/Programming").unwrap(),
               "/home/undeadleech/Programming");
}

// Finally something that doesn't rely on anything
#[test]
fn resolve_root_path() {
    assert_eq!(resolve_path("/root/test").unwrap(), "/root/test");
}

// This obviously only works on my machine / with my username
#[test]
fn home_dir_is_undeadleech() {
    assert_eq!(get_home_dir().unwrap(), String::from("/home/undeadleech"));
}

// Only checks last part of String to make it independent from compile path
#[test]
fn working_dir_ends_in_dotfiler_debug_deps_dir() {
    let working_dir = get_working_dir().unwrap();
    assert_eq!(working_dir.ends_with("dotfiler/target/debug/deps"), true);
}
