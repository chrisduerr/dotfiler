use tera::{Tera, Context};
use toml::{Table, Value};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::Path;

pub fn load(home_dir: &str, app_dir: &str, config: &Table) {
    let variables = match config.get("variables").and_then(Value::as_table) {
        Some(t) => t,
        None => {
            println!("[variables] section is missing or invalid.");
            return;
        },
    };
    let mut context = Context::new();

    for (key, val) in variables {
        match val.as_str() {
            Some(val_str) => context.add(key, &val_str),
            None => println!("Failed parsing {}: Value is not a valid string.",
                             key),
        };
    };

    let tera = Tera::new(format!("{}/templates/*", app_dir).as_str());

    let templates = match config.get("templates").and_then(Value::as_table) {
        Some(t) => t,
        None => {
            println!("[templates] section is missing or invalid.");
            return;
        },
    };

    let mut failed = Vec::new();
    for (template, path) in templates {
        let path = match path.as_str() {
            Some(s) => s,
            None => {
                failed.push((template, String::from("Value is not a valid string.")));
                continue;
            },
        };
        let path = path.replace("$HOME", home_dir).replace("~", home_dir);
        let render = match tera.render(template, context.clone()) {
            Ok(r) => r,
            Err(_) => {
                failed.push((template, String::from("Unable to convert template.")));
                continue;
            },
        };

        if create_dir_all(match Path::new(&path).parent() {
            Some(parent_path) => parent_path,
            None => {
                failed.push((template, String::from("Target directory can't \
                                                    be root.")));
                continue;
            },
        }).is_err() {
            failed.push((template, String::from("Could not create one or \
                                                more directories required.")));
            continue;
        }

        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(_) => {
                failed.push((template, String::from("Couldn't create target file.")));
                continue;
            }
        };
        let _ = file.write_all(render.as_bytes());
    }

    for failure in failed {
        println!("Failed copying {}: {}", failure.0, failure.1);
    }
}
