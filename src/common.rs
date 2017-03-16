use std::io::{self, Read, Write};
use std::{fs, env, path};
use toml::{self, value};
use handlebars;
use rusqlite;

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

pub struct Directory {
    pub directories: Vec<Directory>,
    pub files: Vec<Box<File>>,
}

pub trait File {
    fn save(&self) -> Result<(), error::DotfilerError>;
    fn render(&mut self, &value::Table) -> Result<(), error::DotfilerError>;
    fn template(&mut self, &value::Table) -> Result<(), error::DotfilerError>;
}

struct TextFile {
    pub data: String,
    pub target_path: String,
    _p: (),
}

impl TextFile {
    fn new(file_path: &str, target_path: &str) -> Result<TextFile, error::DotfilerError> {
        let mut data = String::new();
        fs::File::open(file_path)?.read_to_string(&mut data)?;

        Ok(TextFile {
            data: data,
            target_path: target_path.to_string(),
            _p: (),
        })
    }
}

impl File for TextFile {
    fn save(&self) -> Result<(), error::DotfilerError> {
        // Read existing data (if file exists already) into memory for ability to restore
        let mut buf = String::new();
        let res = fs::File::open(&self.target_path).and_then(|mut f| f.read_to_string(&mut buf));
        if let Err(err) = res {
            if err.kind() != io::ErrorKind::NotFound {
                // Tell dad what happened if there was an error while reading existing file
                Err(err)?;
            }
        }

        // Save the file discarding all old data
        if let Err(err) = fs::File::create(&self.target_path)
            .and_then(|mut f| f.write_all(&self.data.as_bytes())) {
            // Restore data and exit with error
            if buf == String::new() {
                // Delete file if it didn't exist until now
                fs::remove_file(&self.target_path)?;
            } else {
                fs::File::create(&self.target_path).and_then(|mut f| f.write_all(buf.as_bytes()))?;
            }

            // Output error after restoring state
            Err(err)?;
        }

        Ok(())
    }

    fn render(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        let handlebars = handlebars::Handlebars::new();
        self.data = handlebars.template_render(&self.data, variables)?;

        Ok(())
    }

    fn template(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        for (key, val) in variables {
            let val_str = val.as_str()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput,
                                   format!("Variable \"{}\" is not a String.", key))
                })?;
            self.data = self.data.replace(val_str, &format!("{{{{ {} }}}}", key));
        }

        Ok(())
    }
}

// Apply one single &str -> String method to every element in the DB
pub fn modify_sqlite_elements(path: &str,
                              func: fn(&str, &value::Table) -> Result<String, error::DotfilerError>,
                              variables: &value::Table)
                              -> Result<(), error::DotfilerError> {
    let mut db_conn = rusqlite::Connection::open(&path)?;
    let transaction = db_conn.transaction()?;

    // Start safe modification of database
    {
        let mut stmt =
            transaction.prepare("SELECT tbl_name FROM sqlite_master WHERE type = 'table'")?;
        let mut tables = stmt.query(&[])?;

        while let Some(Ok(table)) = tables.next() {
            let table: String = match table.get_checked(0) {
                Ok(table) => table,
                Err(_) => continue,
            };

            // Use format because apparently this doesn't work with rusqlite and '?'
            let mut stmt = transaction.prepare(&format!("PRAGMA table_info({})", table))?;
            let mut columns = stmt.query(&[])?;

            while let Some(Ok(column)) = columns.next() {
                let column: String = match column.get_checked(1) {
                    Ok(column) => column,
                    Err(_) => continue,
                };

                let mut stmt = transaction.prepare(&format!("SELECT {} FROM {}", column, table))?;
                let mut current_entries = stmt.query(&[])?;

                while let Some(Ok(current_entry)) = current_entries.next() {
                    let mut current_entry: String = match current_entry.get_checked(0) {
                        Ok(current_entry) => current_entry,
                        Err(_) => continue,
                    };
                    current_entry = current_entry.replace("'", "''");

                    let new_entry = func(&current_entry, variables)?;
                    transaction.execute(&format!("UPDATE {} SET {}='{}' WHERE {}='{}'",
                                          &table,
                                          &column,
                                          &new_entry,
                                          &column,
                                          &current_entry),
                                 &[])?;
                }
            }
        }
    }

    // Rollback if any error occured
    Ok(transaction.commit()?)
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
