use rustc_serialize::json::{Json, ToJson};
use toml::{Parser, Table, Value};
use std::env::{home_dir, current_exe};
use std::fs::{File, create_dir_all};
use std::collections::BTreeMap;
use std::path::Path;
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
            None
        }
    }
}

pub fn get_variables_json() -> Option<Json> {
    let variables = match load_from_toml("variables") {
        Some(t) => t,
        None => return None,
    };

    let mut variables_json: BTreeMap<String, Json> = BTreeMap::new();
    for (key, val) in variables {
        let val = match val.as_str() {
            Some(val) => val,
            None => return None,
        };
        variables_json.insert(key, val.to_json());
    }

    Some(variables_json.to_json())
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
        None => None,
    }
}

pub fn create_directories_for_file(file_path: &str) -> bool {
    let create_dir_path = match Path::new(&file_path).parent() {
        Some(p) => p,
        None => return false,
    };
    create_dir_all(&create_dir_path).is_ok()
}
