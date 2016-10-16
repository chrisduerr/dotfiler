use tera::Context;
use toml::{Parser, Table, Value};
use std::env::{home_dir, current_exe};
use std::path::Path;
use std::fs::{File, create_dir_all};
use std::io::Read;

pub fn load_from_toml(section: &str) -> Option<Table> {
    let config = match get_config() {
        Some(c) => c,
        None => return None,
    };

    match config.get(section).and_then(Value::as_table) {
        Some(t) => Some(t.clone()),
        None => {
            println!("{} section is missing or invalid.", section);
            return None;
        }
    }
}

pub fn get_variables_context() -> Option<Context> {
    let variables = match load_from_toml("variables") {
        Some(t) => t,
        None => return None,
    };

    let mut context = Context::new();
    for (key, val) in variables {
        match val.as_str() {
            Some(val_str) => context.add(&key, &val_str),
            None => {
                println!("Failed parsing {}: Value is not a valid String.", key);
                return None;
            }
        };
    }
    Some(context)
}

pub fn path_value_to_string(path: &Value) -> Option<String> {
    let path_str = match path.as_str() {
        Some(s) => s,
        None => return None,
    };

    let home_dir = get_home_dir();
    Some(path_str.replace("$HOME", &home_dir).replace("~", &home_dir))
}

pub fn get_home_dir() -> String {
    let home_dir = home_dir().unwrap();
    home_dir.to_str().unwrap().to_string()
}

pub fn get_app_dir() -> String {
    let mut app_dir = current_exe().unwrap();
    app_dir.pop();
    app_dir.to_str().unwrap().to_string()
}

pub fn get_config() -> Option<Table> {
    let mut buffer = String::new();
    let _ = match File::open(format!("{}/config.toml", get_app_dir()).as_str()) {
        Ok(mut f) => f.read_to_string(&mut buffer),
        Err(_) => return None,
    };

    match Parser::new(&buffer).parse() {
        Some(config) => Some(config),
        None => return None,
    }
}

pub fn create_directories_for_file(file_path: &str) -> bool {
    let create_dir_path = match Path::new(&file_path).parent() {
        Some(p) => p,
        None => return false,
    };
    create_dir_all(&create_dir_path).is_ok()
}
