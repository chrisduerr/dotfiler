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
        dotfiles: Vec::new(),
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
