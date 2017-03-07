use std::io::{self, Read, Write};
use std::{fs, path, env};
use std::os::unix;
use handlebars;
use rusqlite;
use walkdir;
use toml;

use error;

#[derive(Deserialize)]
pub struct Config {
    pub templates: Option<Vec<Dotfile>>,
    pub variables: toml::Value,
}

#[derive(Deserialize)]
pub struct Dotfile {
    pub source: String,
    pub target: String,
}

// TODO:
// * Implement SQLite dotfiling
// * The "load" method is not tested yet
pub fn load(target_path: &str,
            config_path: &str,
            copy_files: bool,
            copy_sqlite: bool)
            -> Result<(), error::DotfilerError> {
    let config = load_config(config_path)?;
    let templates_dir = path::Path::new(&config_path)
        .parent()
        .ok_or(io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
        .join("templates");

    if let Some(templates) = config.templates {
        for template in templates {
            let src_str = templates_dir.join(&template.source).to_string_lossy().to_string();
            let src_path = resolve_path(&src_str)?;
            let tar_path = [target_path, &resolve_path(&template.target)?[1..]].concat();
            load_file(&src_path,
                      &tar_path,
                      copy_files,
                      copy_sqlite,
                      &config.variables)?;
        }
    }

    Ok(())
}

pub fn get_working_dir() -> Result<String, io::Error> {
    let mut app_dir = env::current_exe()?;
    app_dir.pop();
    Ok(app_dir.to_string_lossy().to_string())
}

fn load_file(src_path: &str,
             tar_path: &str,
             copy_files: bool,
             copy_sqlite: bool,
             variables: &toml::Value)
             -> Result<(), error::DotfilerError> {
    for file in walkdir::WalkDir::new(src_path).into_iter().filter_map(|e| e.ok()) {
        let file_src_path = file.path().to_string_lossy().to_string();
        let file_tar_path = [tar_path, &file_src_path[src_path.len()..]].concat();

        let file_meta = fs::symlink_metadata(&file_src_path)?;

        // Create directories if current element is somethig that needs to be copied
        if file_meta.is_file() || file_meta.file_type().is_symlink() {
            // If there is no parent the file sits in root and that always exists
            if let Some(parent_path) = path::Path::new(&file_tar_path).parent() {
                fs::create_dir_all(&parent_path)?;
            }
        }

        if file_meta.is_file() {
            if is_sqlite(&file_src_path)? && copy_sqlite {
                template_sqlite(&file_src_path, &file_tar_path, variables)?;
            } else if copy_files {
                template_file(&file_src_path, &file_tar_path, variables)?;
            }
        } else if file_meta.file_type().is_symlink() {
            // Remove file because overwriting smylinks is impossible
            if let Err(err) = fs::remove_file(&file_tar_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    Err(err)?;
                }
            }
            let symlink_tar_path = fs::read_link(&file_src_path)?;
            unix::fs::symlink(&symlink_tar_path, &file_tar_path)?;
        }
    }

    Ok(())
}

fn template_file(src_path: &str,
                 tar_path: &str,
                 variables: &toml::Value)
                 -> Result<(), error::DotfilerError> {
    let mut file_content = String::new();
    fs::File::open(src_path)?.read_to_string(&mut file_content)?;

    let handlebars = handlebars::Handlebars::new();
    let templated_file = handlebars.template_render(&file_content, variables)?;

    let mut f = fs::File::create(&tar_path)?;
    f.write_all(templated_file.as_bytes())?;
    f.sync_all()?;

    Ok(())
}

// TODO: Make "?" work, never use format!!!
fn template_sqlite(db_src_path: &str,
                   db_tar_path: &str,
                   variables: &toml::Value)
                   -> Result<(), error::DotfilerError> {
    // Copy the original db to the target location
    fs::copy(db_src_path, db_tar_path)?;

    let db_conn = rusqlite::Connection::open(&db_tar_path)?;
    let mut stmt = db_conn.prepare("SELECT tbl_name FROM sqlite_master WHERE type = 'table'")?;
    let mut tables = stmt.query(&[])?;

    while let Some(Ok(table)) = tables.next() {
        let table: String = match table.get_checked(0) {
            Ok(table) => table,
            Err(_) => continue,
        };

        // Use format because apparently this doesn't work with rusqlite and '?'
        let mut stmt = db_conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let mut columns = stmt.query(&[])?;

        while let Some(Ok(column)) = columns.next() {
            let column: String = match column.get_checked(1) {
                Ok(column) => column,
                Err(_) => continue,
            };

            let mut stmt = db_conn.prepare(&format!("SELECT {} FROM {}", column, table))?;
            let mut current_entries = stmt.query(&[])?;

            while let Some(Ok(current_entry)) = current_entries.next() {
                let mut current_entry: String = match current_entry.get_checked(0) {
                    Ok(current_entry) => current_entry,
                    Err(_) => continue,
                };
                current_entry = current_entry.replace("'", "''");

                let handlebars = handlebars::Handlebars::new();
                let new_entry = handlebars.template_render(&current_entry, variables)?;

                db_conn.execute(&format!("UPDATE {} SET {}='{}' WHERE {}='{}'",
                                      &table,
                                      &column,
                                      &new_entry,
                                      &column,
                                      &current_entry),
                             &[])?;
            }
        }
    }

    Ok(())
}

fn is_sqlite(path: &str) -> Result<bool, io::Error> {
    let mut f = fs::File::open(path)?;
    if f.metadata()?.len() < 6 {
        return Ok(false);
    }

    let mut buffer = [0; 6];
    f.read_exact(&mut buffer)?;

    if String::from_utf8_lossy(&buffer) == String::from("SQLite") {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn load_config(config_path: &str) -> Result<Config, error::DotfilerError> {
    let config_path = resolve_path(config_path)?;
    let mut buffer = String::new();
    fs::File::open(config_path)?.read_to_string(&mut buffer)?;
    Ok(toml::from_str(&buffer)?)
}

// Rust can't deal with "~", "$HOME" or relative paths, this takes care of that
fn resolve_path(path: &str) -> Result<String, error::DotfilerError> {
    if path.starts_with("$HOME") {
        Ok(get_home_dir()? + &path[5..])
    } else if path.starts_with("~") {
        Ok(get_home_dir()? + &path[1..])
    } else {
        Ok(path.to_string())
    }
}

fn get_home_dir() -> Result<String, io::Error> {
    let home_dir = env::home_dir()
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "Unable to locate home directory."))?;
    Ok(home_dir.to_string_lossy().to_string())
}



// -------------
//     TESTS
// -------------

#[cfg(test)]
#[derive(Serialize)]
pub struct World {
    pub world: String,
}

#[cfg(test)]
fn setup(src_path: &str) -> toml::Value {
    let input = "Hello, {{ world }}!";
    let mut f = fs::File::create(src_path).unwrap();
    f.write_all(&input.as_bytes()).unwrap();
    f.sync_all().unwrap();

    let var = World { world: String::from("world") };
    toml::Value::try_from(var).unwrap()
}

#[cfg(test)]
fn teardown(src_path: &str, tar_path: &str) {
    fs::remove_file(src_path).unwrap();
    fs::remove_file(tar_path).unwrap();
}

#[test]
fn is_sqlite_with_non_sqlite_file_is_true() {
    assert_eq!(is_sqlite("./examples/templates/db.sqlite").unwrap(), true);
}

#[test]
fn is_sqlite_with_non_sqlite_file_is_false() {
    assert_eq!(is_sqlite("./examples/templates/Xresources").unwrap(), false);
}

#[test]
fn is_sqlite_with_non_existing_file_is_error() {
    assert_eq!(is_sqlite("./this/doesnt/exist").is_err(), true);
}

#[test]
fn template_file_correctly_rendering_string_to_text() {
    let src_path = "./template_file_correctly_rendering_string_to_text_src";
    let tar_path = "./template_file_correctly_rendering_string_to_text_tar";

    let val = setup(src_path);

    template_file(src_path, tar_path, &val).unwrap();

    let mut output = String::new();
    fs::File::open(tar_path).unwrap().read_to_string(&mut output).unwrap();

    teardown(src_path, tar_path);

    assert_eq!(output, String::from("Hello, world!"));
}

#[test]
fn load_file_correctly_loading_single_file() {
    let src_path = "./load_file_correctly_loading_single_file_src";
    let tar_path = "./load_file_correctly_loading_single_file_tar";

    let val = setup(src_path);

    load_file(src_path, tar_path, true, true, &val).unwrap();

    let mut output = String::new();
    fs::File::open(tar_path).unwrap().read_to_string(&mut output).unwrap();

    teardown(src_path, tar_path);

    assert_eq!(output, String::from("Hello, world!"));
}

#[test]
fn load_file_creating_directories() {
    let src_path = "./load_file_creating_directories_src";
    let tar_path = "./load_file_creating_directories_tar";

    let val = setup(src_path);

    load_file(src_path, tar_path, true, true, &val).unwrap();

    let mut output = String::new();
    fs::File::open(tar_path).unwrap().read_to_string(&mut output).unwrap();

    teardown(src_path, tar_path);

    assert_eq!(output, String::from("Hello, world!"));
}

#[test]
fn load_correctly_saving_example_to_dummy_dir() {
    load("./dummy/",
         "/home/undeadleech/Programming/Rust/dotfiler/examples/config.toml",
         true,
         true)
        .unwrap();

    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/MyXresourcesTemplate").is_ok(),
               true);
    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/Scripts").is_ok(),
               true);
    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/db.sqlite").is_ok(),
               true);

    // fs::remove_dir_all("./dummy/").unwrap(); // Make sure dir always is deleted
}

// This obviously only works on my machine / with my username
#[test]
fn home_dir_is_undeadleech() {
    assert_eq!(get_home_dir().unwrap(), String::from("/home/undeadleech"));
}

// This doesn't even work if you're called undeadleech
// You need the exact same folder structure I have
#[test]
fn working_dir_is_programming_rust_dotfiler() {
    assert_eq!(get_working_dir().unwrap(),
               String::from("/home/undeadleech/Programming/Rust/dotfiler/target/debug/deps"));
}

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
