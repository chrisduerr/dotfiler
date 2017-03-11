use std::io::{self, Read};
use toml::{self, value};
use std::{fs, env};
use rusqlite;

use error;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub templates: Option<Vec<Dotfile>>,
    pub variables: value::Table,
}

#[derive(Serialize, Deserialize)]
pub struct Dotfile {
    pub template: String,
    pub target: String,
}

// Apply one single &str -> String method to every element in the DB
pub fn modify_sqlite_elements(path: &str,
                              func: fn(&str, &value::Table) -> Result<String, error::DotfilerError>,
                              variables: &value::Table)
                              -> Result<(), error::DotfilerError> {
    let db_conn = rusqlite::Connection::open(&path)?;
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

                let new_entry = func(&current_entry, variables)?;
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

pub fn load_config(config_path: &str) -> Result<Config, error::DotfilerError> {
    let config_path = resolve_path(config_path)?;
    let mut buffer = String::new();
    fs::File::open(config_path)?.read_to_string(&mut buffer)?;
    Ok(toml::from_str(&buffer)?)
}

// Rust can't deal with "~", "$HOME" or relative paths, this takes care of that
pub fn resolve_path(path: &str) -> Result<String, error::DotfilerError> {
    if path.starts_with("$HOME") {
        Ok(get_home_dir()? + &path[5..])
    } else if path.starts_with('~') {
        Ok(get_home_dir()? + &path[1..])
    } else {
        Ok(path.to_string())
    }
}

pub fn get_home_dir() -> Result<String, io::Error> {
    let home_dir =
        env::home_dir().ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "Unable to locate home directory.")
            })?;
    Ok(home_dir.to_string_lossy().to_string())
}

pub fn is_sqlite(path: &str) -> Result<bool, io::Error> {
    let mut f = fs::File::open(path)?;
    if f.metadata()?.len() < 6 {
        return Ok(false);
    }

    let mut buffer = [0; 6];
    f.read_exact(&mut buffer)?;

    if String::from_utf8_lossy(&buffer) == "SQLite" {
        Ok(true)
    } else {
        Ok(false)
    }
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
