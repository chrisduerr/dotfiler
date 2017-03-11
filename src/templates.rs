use std::io::{self, Read, Write};
use std::{fs, path, env};
use toml::{self, value};
use std::os::unix;
use handlebars;
use walkdir;

use common;
use error;

// TODO: Rename source to template
// TODO: Don't just die if target or template directories could not be found
pub fn load(target_path: &str,
            config_path: &str,
            copy_files: bool,
            copy_sqlite: bool)
            -> Result<(), error::DotfilerError> {
    let config = common::load_config(config_path)?;
    let templates_dir = path::Path::new(config_path)
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
        .join("templates");

    if let Some(ref templates) = config.templates {
        for template in templates {
            let src_str = templates_dir.join(&template.template).to_string_lossy().to_string();
            let src_path = common::resolve_path(&src_str)?;
            let tar_path = [target_path, &common::resolve_path(&template.target)?[1..]].concat();
            load_file(&src_path,
                      &tar_path,
                      copy_files,
                      copy_sqlite,
                      &config.variables)?;
        }
    }

    // Save the config as old.toml
    let config_str = toml::to_string(&config)?;
    let old_config_path = templates_dir.join("old.toml").to_string_lossy().to_string();
    fs::File::create(&old_config_path)?.write_all(config_str.as_bytes())?;

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
             variables: &value::Table)
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
            if common::is_sqlite(&file_src_path)? && copy_sqlite {
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
                 variables: &value::Table)
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
                   variables: &value::Table)
                   -> Result<(), error::DotfilerError> {
    // Copy the original db to the target location
    fs::copy(db_src_path, db_tar_path)?;

    fn modify(entry: &str, variables: &value::Table) -> Result<String, error::DotfilerError> {
        let handlebars = handlebars::Handlebars::new();
        Ok(handlebars.template_render(entry, variables)?)
    };

    common::modify_sqlite_elements(db_tar_path, modify, variables)
}



// -------------
//     TESTS
// -------------

#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(test)]
fn setup(src_path: &str) -> value::Table {
    let input = "Hello, {{ world }}!";
    let mut f = fs::File::create(src_path).unwrap();
    f.write_all(&input.as_bytes()).unwrap();
    f.sync_all().unwrap();

    let mut map = BTreeMap::new();
    map.insert(String::from("world"),
               toml::Value::String(String::from("World")));
    map
}

#[cfg(test)]
fn teardown(src_path: &str, tar_path: &str) {
    fs::remove_file(src_path).unwrap();
    fs::remove_file(tar_path).unwrap();
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

    assert_eq!(output, String::from("Hello, World!"));
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

    assert_eq!(output, String::from("Hello, World!"));
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

    assert_eq!(output, String::from("Hello, World!"));
}

#[test]
fn load_correctly_saving_example_to_dummy_dir() {
    load("./dummy/",
         "/home/undeadleech/Programming/Rust/dotfiler/examples/config.toml",
         true,
         true)
        .unwrap();

    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/Xresources").is_ok(),
               true);
    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/Scripts").is_ok(),
               true);
    assert_eq!(fs::metadata("./dummy/home/undeadleech/testing/db.sqlite").is_ok(),
               true);

    // fs::remove_dir_all("./dummy/").unwrap(); // Make sure dir always is deleted
}

// This doesn't even work if you're called undeadleech
// You need the exact same folder structure I have
#[test]
fn working_dir_is_programming_rust_dotfiler() {
    assert_eq!(get_working_dir().unwrap(),
               String::from("/home/undeadleech/Programming/Rust/dotfiler/target/debug/deps"));
}
