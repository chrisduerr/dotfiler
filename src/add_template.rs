use std::io::{self, Read, Write};
use std::{fs, path};
use std::os::unix;
use toml::value;
use walkdir;
use toml;

use common;
use error;

// TODO: Don't make any changes if one file template fails
// TODO: Don't edit the config if one file fails
// TODO: Implement option to rename file/directory (flag?)
pub fn add_template(config_path: &str,
                    file_path: &str,
                    new_name: Option<&str>,
                    templating_enabled: bool)
                    -> Result<(), error::DotfilerError> {
    let mut config = common::load_config(config_path)?;
    let templates_path = common::get_templates_path(config_path)?;
    let tar_prefix = path::Path::new(file_path)
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
        .to_string_lossy();

    for file in walkdir::WalkDir::new(file_path).into_iter().filter_map(|e| e.ok()) {
        let file_tar_path = file.path().to_string_lossy().to_string();
        let file_template_path = templates_path.join(&file_tar_path[tar_prefix.len() + 1..])
            .to_string_lossy()
            .to_string();

        if template_exists_already(&config.dotfiles, &file_template_path, &file_tar_path)? {
            Err(io::Error::new(io::ErrorKind::AlreadyExists, "Dotfile already exists."))?;
        }

        let file_meta = match fs::symlink_metadata(&file_path) {
            Ok(meta) => meta,
            Err(e) => {
                println!("Unable to get metadata for {}\n{}", file_path, e);
                continue;
            }
        };

        {
            // Get dirs required for this file to exist, if directory get itself
            let mut required_dirs = path::Path::new(&file_template_path);
            if file_meta.is_file() || file_meta.file_type().is_symlink() {
                if let Some(dirs) = required_dirs.parent() {
                    required_dirs = dirs;
                }
            }

            // If there is no parent the file sits in root and that always exists
            if let Err(e) = fs::create_dir_all(&required_dirs) {
                println!("Unable to create directories {:?}\n{}", required_dirs, e);
            }
        }

        if file_meta.is_file() {
            if templating_enabled {
                if let Ok(true) = common::is_sqlite(&file_tar_path) {
                    if let Err(e) = create_sqlite_template(&file_template_path,
                                                           &file_tar_path,
                                                           &config.variables) {
                        println!("Unable to template SQLite file '{}' to '{}'\n{}",
                                 file_tar_path,
                                 file_template_path,
                                 e);
                    }
                } else if let Err(e) = create_file_template(&file_template_path,
                                                            &file_tar_path,
                                                            &config.variables) {
                    println!("Unable to template file '{}' to '{}'\n{}",
                             file_tar_path,
                             file_template_path,
                             e);
                }
            } else if let Err(e) = fs::copy(&file_tar_path, file_template_path) {
                println!("Unable to copy file {}\n{}", file_tar_path, e);
            }
        } else if file_meta.file_type().is_symlink() {
            // Remove file because overwriting smylinks is impossible
            if let Err(err) = fs::remove_file(&file_template_path) {
                if err.kind() != io::ErrorKind::NotFound {
                    println!("Unable to delete symlink {}\n{}", file_template_path, err);
                }
            }

            let symlink_tar_path = match fs::read_link(&file_tar_path) {
                Ok(sym) => sym,
                Err(e) => {
                    println!("Unable to read symlink {}\n{}", file_tar_path, e);
                    continue;
                }
            };

            if let Err(e) = unix::fs::symlink(&symlink_tar_path, &file_template_path) {
                println!("Unable to create symlink {}\n{}", file_tar_path, e);
            }
        }
    }

    let template_path = templates_path.join(&file_path[tar_prefix.len() + 1..])
        .to_string_lossy()
        .to_string();
    let dotfile = common::Dotfile {
        template: template_path,
        target: file_path.to_string(),
    };

    config.dotfiles.push(dotfile);

    let new_config_content = toml::to_string(&config)?;
    fs::File::create(&config_path)?.write_all(new_config_content.as_bytes())?;

    Ok(())
}

fn template_exists_already(dotfiles: &Vec<common::Dotfile>,
                           template_path: &str,
                           tar_path: &str)
                           -> Result<bool, error::DotfilerError> {
    for dotfile in dotfiles {
        let existing_template_path = common::resolve_path(&dotfile.template)?;
        let existing_tar_path = common::resolve_path(&dotfile.target)?;
        if existing_template_path == template_path && existing_tar_path == tar_path {
            return Ok(true);
        }
    }

    Ok(false)
}

fn create_file_template(template_path: &str,
                        tar_path: &str,
                        variables: &value::Table)
                        -> Result<(), error::DotfilerError> {
    let mut content = String::new();
    fs::File::open(tar_path)?.read_to_string(&mut content)?;

    for (key, val) in variables {
        let val_str = val.as_str()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput,
                               format!("Variable \"{}\" is not a String.", key))
            })?;
        content = content.replace(val_str, &format!("{{{{ {} }}}}", key));
    }

    Ok(fs::File::create(template_path)?.write_all(content.as_bytes())?)
}

fn create_sqlite_template(template_path: &str,
                          tar_path: &str,
                          variables: &value::Table)
                          -> Result<(), error::DotfilerError> {
    fs::copy(tar_path, template_path)?;

    fn modify(entry: &str, variables: &value::Table) -> Result<String, error::DotfilerError> {
        let mut new_entry = entry.to_owned();
        for (key, val) in variables {
            if let Some(val) = val.as_str() {
                new_entry = new_entry.replace(&val, &format!("{{{{ {} }}}}", key));
            }
        }
        Ok(new_entry)
    };

    common::modify_sqlite_elements(template_path, modify, variables)
}



// -------------
//     TESTS
// -------------

#[cfg(test)]
use std::collections::BTreeMap;

#[test]
fn directories_are_copied_correctly() {
    let tar_path = "directories_are_copied_correctly_tar";
    let template_path = "templates/directories_are_copied_correctly_tar";

    fs::create_dir_all([tar_path, "/xyz"].concat()).unwrap();

    let config = common::Config {
        dotfiles: None,
        variables: BTreeMap::new(),
    };
    let config_content = toml::to_string(&config).unwrap();
    fs::File::create("tmp_config.toml").unwrap().write_all(config_content.as_bytes()).unwrap();

    add_template("tmp_config.toml", tar_path, None, false).unwrap();

    let dir_exists = fs::metadata([template_path, "/xyz"].concat()).is_ok();

    let _ = fs::remove_file("tmp_config.toml");
    let _ = fs::remove_dir_all("directories_are_copied_correctly_tar");
    let _ = fs::remove_dir_all("templates");

    assert_eq!(dir_exists, true);
}
