use std::io::{self, Read, Write};
use std::{fs, path};
use std::os::unix;
use toml::value;
use handlebars;
use tempfile;
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
        return Ok(Box::new(TextFile::new(src_path, tar_path)?));
    } else if filetype.is_symlink() {
        return Ok(Box::new(Symlink::new(src_path, tar_path)?));
    }

    Ok(Err(io::Error::new(io::ErrorKind::InvalidData, "FileType does not exist."))?)
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
    old_data: String,
    target_path: String,
    existed_already: bool,
}

impl TextFile {
    fn new(file_path: &str, target_path: &str) -> Result<TextFile, error::DotfilerError> {
        let mut data = String::new();
        fs::File::open(file_path)?.read_to_string(&mut data)?;

        Ok(TextFile {
               data: data,
               old_data: String::new(),
               target_path: target_path.to_string(),
               existed_already: true,
           })
    }
}

impl File for TextFile {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        // Read existing data (if file exists already) into memory for ability to restore
        let file_result = fs::File::open(&self.target_path);
        match file_result.and_then(|mut f| f.read_to_string(&mut self.old_data)) {
            Ok(_) => {
                self.existed_already = true;
            }
            Err(err) => {
                match err.kind() {
                    io::ErrorKind::NotFound => self.existed_already = false,
                    _ => Err(err)?,
                }
            }
        };

        fs::File::create(&self.target_path).and_then(|mut f| f.write_all(&self.data.as_bytes()))?;

        Ok(())
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        if !self.existed_already {
            fs::remove_file(&self.target_path)?;
        } else {
            fs::File::create(&self.target_path).and_then(|mut f| f.write_all(&self.old_data.as_bytes()))?;
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
        let backup_path = &["./cache", file_path].concat();
        if let Err(error::DotfilerError::IoError(e)) = copy_symlink(file_path, backup_path) {
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
    data: tempfile::NamedTempFile,
    target_path: String,
    existed_already: bool,
}

impl SQLite {
    fn new(file_path: &str, target_path: &str) -> Result<SQLite, error::DotfilerError> {
        unimplemented!();
    }
}

impl File for SQLite {
    fn save(&mut self) -> Result<(), error::DotfilerError> {
        unimplemented!();
    }

    fn restore(&self) -> Result<(), error::DotfilerError> {
        unimplemented!();
    }

    fn render(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        unimplemented!();
    }

    fn template(&mut self, variables: &value::Table) -> Result<(), error::DotfilerError> {
        unimplemented!();
    }
}

// Copy a symlink overwriting any existing file at "tar"
fn copy_symlink(src: &str, tar: &str) -> Result<(), error::DotfilerError> {
    // Read src target link
    let src_target = fs::read_link(src)?.to_string_lossy().to_string();

    // Create required directories
    let parent_path = path::Path::new(tar).parent().ok_or(String::from("Cannot symlink to root."))?;
    fs::create_dir_all(&parent_path.to_string_lossy().to_string())?;

    // Try delete old symlink because symlinks can't be overwritten
    let _ = fs::remove_file(tar);

    // Create new symlink
    Ok(unix::fs::symlink(&src_target, tar).unwrap())
}
