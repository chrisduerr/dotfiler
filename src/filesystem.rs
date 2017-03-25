use std::io::{self, Read, Write};
use std::{fs, path};
use std::os::unix;
use toml::value;
use handlebars;
use rusqlite;
use walkdir;

use common;
use error;

// TODO: Add SQLite templating again!

pub fn create_tree_from_path(src_path: &str,
                             tar_path: &str)
                             -> Result<Box<File>, error::DotfilerError> {
    let src_path = common::resolve_path(src_path)?;
    let tar_path = common::resolve_path(tar_path)?;

    let filetype = fs::symlink_metadata(&src_path)?.file_type();
    Ok(file_from_filetype(&filetype, &src_path, &tar_path)?)
}

fn file_from_filetype(filetype: &fs::FileType,
                      src_path: &str,
                      tar_path: &str)
                      -> Result<Box<File>, error::DotfilerError> {
    if filetype.is_dir() {
        return Ok(Box::new(Directory::new(src_path, tar_path)?));
    } else if filetype.is_file() {
        if is_sqlite(src_path)? {
            return Ok(Box::new(SQLite::new(src_path, tar_path)?));
        } else if is_binary(src_path) {
            return Ok(Box::new(BinaryFile::new(src_path, tar_path)?));
        } else {
            return Ok(Box::new(TextFile::new(src_path, tar_path)?));
        }
    } else if filetype.is_symlink() {
        return Ok(Box::new(Symlink::new(src_path, tar_path)?));
    }

    Ok(Err(io::Error::new(io::ErrorKind::InvalidData, "FileType does not exist."))?)
}

fn is_binary(path: &str) -> bool {
    let mut buf = String::new();
    if let Err(e) = fs::File::open(path).and_then(|mut f| f.read_to_string(&mut buf)) {
        if e.kind() == io::ErrorKind::InvalidData {
            return true;
        }
    }

    false
}

fn is_sqlite(path: &str) -> Result<bool, io::Error> {
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

pub trait File {
    fn save(&mut self) -> Result<(), error::DotfilerError>;
    fn restore(&self) -> Result<(), error::DotfilerError>;
    fn render(&mut self, &value::Table) -> Result<(), error::DotfilerError>;
    fn template(&mut self, &value::Table) -> Result<(), error::DotfilerError>;
}

struct Directory {
    files: Vec<Box<File>>,
    target_path: String,
    existed_already: bool,
}

impl Directory {
    fn new(file_path: &str, target_path: &str) -> Result<Directory, error::DotfilerError> {
        let mut files: Vec<Box<File>> = Vec::new();
        for file in walkdir::WalkDir::new(&file_path).into_iter().filter_map(|e| e.ok()) {
            let file_str = file.path().to_string_lossy();
            if file_str != file_path {
                let file_tar_path = [target_path, &file_str[file_path.len()..]].concat();

                // Create specific File for every FileType possible
                let filetype = file.file_type();
                files.push(file_from_filetype(&filetype, &file_str, &file_tar_path)?);
            }
        }

        Ok(Directory {
               files: files,
               target_path: target_path.to_string(),
               existed_already: true,
           })
    }
}

impl File for Directory {
    // First create the directory itself, then children
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        if let Ok(meta) = fs::metadata(&self.target_path) {
            if meta.is_dir() {
                self.existed_already = true;
            } else {
                // Path already exists but is no directory
                let error_msg = format!("The path '{}' already exists but is not a directory.",
                                        self.target_path);
                Err(io::Error::new(io::ErrorKind::AlreadyExists, error_msg))?;
            }
        } else {
            self.existed_already = false;
            fs::create_dir(&self.target_path)?;
        }

        for file in &mut self.files {
            file.save()?;
        }

        Ok(())
    }

    // Remove children first and then this directory
    fn restore(&self) -> Result<(), error::DotfilerError> {
        let mut errors: Vec<Result<(), error::DotfilerError>> = Vec::new();

        for file in &self.files {
            errors.push(file.restore());
        }

        if !self.existed_already {
            fs::remove_dir(&self.target_path)?;
        }

        // Wait for unwrapping until everything is restored
        if !errors.is_empty() {
            errors.swap_remove(0)
        } else {
            Ok(())
        }
    }

    fn render(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        for file in &mut self.files {
            file.render(variables)?;
        }

        Ok(())
    }

    fn template(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        for file in &mut self.files {
            file.template(variables)?;
        }

        Ok(())
    }
}

struct TextFile {
    data: String,
    target_path: String,
    backup_path: String,
    existed_already: bool,
}

impl TextFile {
    fn new(file_path: &str, target_path: &str) -> Result<TextFile, error::DotfilerError> {
        // Create backup path and required directories
        // parent_path can't fail since path is always at least "./cache" -> unwrap
        let backup_path = &["./cache", target_path].concat();
        let parent_path = path::Path::new(&backup_path).parent().unwrap();
        fs::create_dir_all(&parent_path.to_string_lossy().to_string())?;

        let mut existed_already = true;
        if let Err(e) = fs::copy(target_path, backup_path) {
            if e.kind() == io::ErrorKind::InvalidInput {
                existed_already = false;
            } else {
                Err(e)?;
            }
        }

        let mut data = String::new();
        fs::File::open(file_path)?.read_to_string(&mut data)?;

        Ok(TextFile {
               data: data,
               target_path: target_path.to_string(),
               backup_path: backup_path.to_string(),
               existed_already: existed_already,
           })
    }
}

impl File for TextFile {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        fs::File::create(&self.target_path).and_then(|mut f| f.write_all(self.data.as_bytes()))?;

        Ok(())
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        if !self.existed_already {
            fs::remove_file(&self.target_path)?;
        } else {
            let _ = fs::remove_file(&self.target_path);
            fs::copy(&self.backup_path, &self.target_path)?;
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

struct Symlink {
    target: String,
    target_path: String,
    backup_path: String,
    existed_already: bool,
}

impl Symlink {
    fn new(file_path: &str, target_path: &str) -> Result<Symlink, error::DotfilerError> {
        // Copy old symlink to backup location
        let mut existed_already = true;
        let backup_path = &["./cache", target_path].concat();
        if let Err(error::DotfilerError::IoError(e)) = copy_symlink(target_path, backup_path) {
            if e.kind() == io::ErrorKind::NotFound {
                existed_already = false;
            } else {
                Err(e)?;
            }
        }

        let symlink_tar_path = fs::read_link(&file_path)?;

        Ok(Symlink {
               target: symlink_tar_path.to_string_lossy().to_string(),
               target_path: target_path.to_string(),
               backup_path: backup_path.to_string(),
               existed_already: existed_already,
           })
    }
}

impl File for Symlink {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        if self.existed_already {
            fs::remove_file(&self.target_path)?;
        }

        // Create symlink
        Ok(unix::fs::symlink(&self.target, &self.target_path)?)
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        if !self.existed_already {
            fs::remove_file(&self.target_path)?;
        } else {
            let _ = fs::remove_file(&self.target_path);
            copy_symlink(&self.backup_path, &self.target_path)?;
        }

        Ok(())
    }

    // Symlinks are not treated as files but just as links, so no rendering
    fn render(&mut self, _variables: &value::Table) -> Result<(), error::DotfilerError> {
        Ok(())
    }

    // Symlinks are not treated as files but just as links, so no templating
    fn template(&mut self, _variables: &value::Table) -> Result<(), error::DotfilerError> {
        Ok(())
    }
}

struct SQLite {
    target_path: String,
    backup_path: String,
    existed_already: bool,
}

impl SQLite {
    fn new(file_path: &str, target_path: &str) -> Result<SQLite, error::DotfilerError> {
        // Create backup path and required directories
        // parent_path can't fail since path is always at least "./cache" -> unwrap
        let backup_path = &["./cache", target_path].concat();
        let parent_path = path::Path::new(&backup_path).parent().unwrap();
        fs::create_dir_all(&parent_path.to_string_lossy().to_string())?;

        let mut existed_already = true;
        if let Err(e) = fs::copy(target_path, backup_path) {
            if e.kind() == io::ErrorKind::InvalidInput {
                existed_already = false;
            } else {
                Err(e)?;
            }
        }

        // Overwrite the current file with the templated version
        fs::copy(file_path, target_path)?;

        Ok(SQLite {
               target_path: target_path.to_string(),
               backup_path: backup_path.to_string(),
               existed_already: existed_already,
           })
    }
}

impl File for SQLite {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        // Templating and rendering works directly on the target file,
        // this means saving isn't needed
        Ok(())
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        if !self.existed_already {
            fs::remove_file(&self.target_path)?;
        } else {
            let _ = fs::remove_file(&self.target_path);
            fs::copy(&self.backup_path, &self.target_path)?;
        }

        Ok(())
    }

    fn render(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        fn modify(entry: &str, variables: &value::Table) -> Result<String, error::DotfilerError> {
            let handlebars = handlebars::Handlebars::new();
            Ok(handlebars.template_render(entry, variables)?)
        };

        Ok(modify_sqlite_elements(&self.target_path, modify, variables)?)
    }

    fn template(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        fn modify(entry: &str, variables: &value::Table) -> Result<String, error::DotfilerError> {
            let mut new_entry = entry.to_owned();
            for (key, val) in variables {
                if let Some(val) = val.as_str() {
                    new_entry = new_entry.replace(&val, &format!("{{{{ {} }}}}", key));
                }
            }
            Ok(new_entry)
        };

        Ok(modify_sqlite_elements(&self.target_path, modify, variables)?)
    }
}

struct BinaryFile {
    src_path: String,
    target_path: String,
    backup_path: String,
    existed_already: bool,
}

impl BinaryFile {
    fn new(file_path: &str, target_path: &str) -> Result<BinaryFile, error::DotfilerError> {
        // Create backup path and required directories
        // parent_path can't fail since path is always at least "./cache" -> unwrap
        let backup_path = &["./cache", target_path].concat();
        let parent_path = path::Path::new(&backup_path).parent().unwrap();
        fs::create_dir_all(&parent_path.to_string_lossy().to_string())?;

        let mut existed_already = true;
        if let Err(e) = fs::copy(target_path, backup_path) {
            if e.kind() == io::ErrorKind::InvalidInput {
                existed_already = false;
            } else {
                Err(e)?;
            }
        }

        Ok(BinaryFile {
               src_path: file_path.to_string(),
               target_path: target_path.to_string(),
               backup_path: backup_path.to_string(),
               existed_already: existed_already,
           })
    }
}

impl File for BinaryFile {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        fs::copy(&self.src_path, &self.target_path)?;
        Ok(())
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        if !self.existed_already {
            fs::remove_file(&self.target_path)?;
        } else {
            let _ = fs::remove_file(&self.target_path);
            fs::copy(&self.backup_path, &self.target_path)?;
        }

        Ok(())
    }

    fn render(&mut self, _variables: &value::Table) -> Result<(), error::DotfilerError> {
        // Binary files can't be templated or rendered
        Ok(())
    }

    fn template(&mut self, _variables: &value::Table) -> Result<(), error::DotfilerError> {
        // Binary files can't be templated or rendered
        Ok(())
    }
}

// Copy a symlink overwriting any existing file at "tar"
fn copy_symlink(src: &str, tar: &str) -> Result<(), error::DotfilerError> {
    // Read src target link
    let src_target = fs::read_link(src)?.to_string_lossy().to_string();

    // Create required directories
    let parent_path = path::Path::new(tar).parent()
        .ok_or_else(|| String::from("Cannot symlink to root."))?;
    fs::create_dir_all(&parent_path.to_string_lossy().to_string())?;

    // Try delete old symlink because symlinks can't be overwritten
    let _ = fs::remove_file(tar);

    // Create new symlink
    Ok(unix::fs::symlink(&src_target, tar).unwrap())
}

// Apply one single &str -> String method to every element in a SQLite DB
fn modify_sqlite_elements(path: &str,
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
