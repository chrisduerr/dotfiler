use std::io::{self, Read, Write};
use std::{fs, path};
use std::os::unix;
use toml::value;
use walkdir;

use common;
use error;

// TODO: Directories not working, copy them and test it
// TODO: Remove this as a general step and make it optional and an explicit command
pub fn create_templates(config_path: &str) -> Result<(), error::DotfilerError> {
    let templates_dir = path::Path::new(config_path)
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
        .join("templates");

    let old_path = templates_dir.join("old.toml").to_string_lossy().to_string();
    let config = match common::load_config(&old_path) {
        Ok(conf) => conf,
        Err(_) => common::load_config(config_path)?,
    };

    if let Some(ref templates) = config.templates {
        for template in templates {
            let src_str = templates_dir.join(&template.template).to_string_lossy().to_string();
            let template_path = common::resolve_path(&src_str)?;
            let tar_path = &common::resolve_path(&template.target)?;

            if common::is_sqlite(tar_path)? {
                create_sqlite_template(&template_path, tar_path, &config.variables)?;
            } else {
                create_file_template(&template_path, tar_path, &config.variables)?;
            }
        }
    }

    Ok(())
}

pub fn create_file_template(template_path: &str,
                            tar_path: &str,
                            variables: &value::Table)
                            -> Result<(), error::DotfilerError> {
    for file in walkdir::WalkDir::new(template_path).into_iter().filter_map(|e| e.ok()) {
        let file_template_path = file.path().to_string_lossy().to_string();
        let file_tar_path = [tar_path, &file_template_path[template_path.len()..]].concat();

        let file_meta = match fs::symlink_metadata(&file_tar_path) {
            Ok(meta) => meta,
            Err(e) => {
                println!("Unable to get metadata for {}\n{}", file_tar_path, e);
                continue;
            }
        };

        // Create directories if current element is somethig that needs to be copied
        if file_meta.is_file() || file_meta.file_type().is_symlink() {
            // If there is no parent the file sits in root and that always exists
            if let Some(parent_path) = path::Path::new(&file_template_path).parent() {
                if let Err(e) = fs::create_dir_all(&parent_path) {
                    println!("Unable to create directories {:?}\n{}", parent_path, e);
                }
            }
        }

        if file_meta.is_file() {
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

            fs::File::create(template_path)?.write_all(content.as_bytes())?;
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
                println!("Unable to create symlink {}\n{}", file_template_path, e);
            }
        }
    }

    Ok(())
}

pub fn create_sqlite_template(template_path: &str,
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
fn create_file_template_correctly_creating_file_template() {
    let tar_path = "create_file_template_correctly_creating_file_template_input";
    let template_path = "create_file_template_correctly_creating_file_template_output";

    fs::File::create(tar_path)
        .unwrap()
        .write_all("test: #123456".as_bytes())
        .unwrap();

    let mut vars = BTreeMap::new();
    vars.insert(String::from("test"),
                value::Value::String(String::from("#123456")));

    create_file_template(template_path, tar_path, &vars).unwrap();

    let mut output = String::new();
    fs::File::open(template_path).unwrap().read_to_string(&mut output).unwrap();

    let _ = fs::remove_file(tar_path);
    let _ = fs::remove_file(template_path);

    assert_eq!(output, "test: {{ test }}");
}

#[test]
fn directories_are_copied_correctly() {
    let tar_path = "directories_are_copied_correctly_tar/";
    let template_path = "directories_are_copied_correctly_template/";

    fs::create_dir_all([tar_path, "/xyz"].concat()).unwrap();
    let vars = BTreeMap::new();

    create_file_template(template_path, tar_path, &vars).unwrap();

    assert_eq!(fs::metadata([template_path, "/xyz"].concat()).is_ok(), true);
}
