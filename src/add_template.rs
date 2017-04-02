use std::io::{self, Write};
use std::{fs, path};
use toml;

use filesystem;
use common;
use error;

pub fn add_template(config_path: &str,
                    file_path: &str,
                    new_name: Option<&str>,
                    templating_enabled: bool)
                    -> Result<(), error::DotfilerError> {
    let mut config = common::load_config(&config_path)?;

    let templates_path = common::get_templates_path(&config_path)?;

    let tar_path = match new_name {
        Some(name) => name,
        None => &file_path[file_path.rfind('/').unwrap() + 1..],
    };
    let tar_path = templates_path.join(tar_path);
    let tar_path = tar_path.to_string_lossy().to_string();

    if let Some(ref mut dotfiles) = config.dotfiles {
        if let Some(duplicate_entry) =
            template_exists_already(&dotfiles,
                                    &templates_path.to_string_lossy(),
                                    &tar_path,
                                    file_path)? {
            println!("The template exists already. Do you want to update or overwrite it? [y/N]");

            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;

            if buf.to_lowercase().trim() != "y" {
                println!("The file has not been added.");
                return Ok(());
            } else {
                dotfiles.swap_remove(duplicate_entry);
                if let Err(e) = fs::remove_dir_all(&tar_path) {
                    let msg = format!("Unable to overwrite current template: {}", e);
                    return Err(error::DotfilerError::Message(msg));
                }
            }
        }
    } else {
        config.dotfiles = Some(Vec::new());
    }

    // Back up old config to cache
    if let Err(e) = fs::copy(&config_path, "./cache/config.toml") {
        let msg = format!("Unable to save current config to backup cache:\n{}", e);
        return Err(error::DotfilerError::Message(msg));
    }

    // Create all required target directories before root
    let _ = path::Path::new(&tar_path).parent().map(|p| fs::create_dir_all(&p));

    let mut root = match filesystem::create_tree_from_path(file_path, &tar_path) {
        Ok(root) => root,
        Err(e) => {
            let msg = format!("Can't create tree for file '{}':\n{}", file_path, e);
            return Err(error::DotfilerError::Message(msg));
        }
    };

    if templating_enabled {
        if let Some(ref vars) = config.variables {
            if let Err(e) = root.template(&vars) {
                let msg = format!("Unable to add the file '{}':\n{}", file_path, e);
                return Err(error::DotfilerError::Message(msg));
            }
        }
    }

    if let Err(e) = root.save() {
        let mut msg = format!("Unable to add the file '{}':\n{}", file_path, e);

        if let Err(e) = root.restore() {
            msg = format!("Critical Error! Unable to recover from failure.\n{}", e);
        }

        return Err(error::DotfilerError::Message(msg));
    }

    // Add new file to config
    let dotfile = common::Dotfile {
        template: tar_path.clone(),
        target: file_path.to_string(),
    };

    if let Some(ref mut dotfiles) = config.dotfiles {
        dotfiles.push(dotfile);
    }

    // Save new config
    let new_config = toml::to_string(&config)?;
    if let Err(e) = fs::File::create(common::resolve_path(&config_path, None)?)
           .and_then(|mut f| f.write_all(new_config.as_bytes())) {
        let mut msg = format!("Unable to save new config:\n{}", e);

        if let Err(e) = fs::copy("./cache/config.toml", &config_path) {
            msg = format!("Unable to restore old config after failure:\n{}", e);
        }

        return Err(error::DotfilerError::Message(msg));
    }

    println!("Successfully added '{}' to dotfiles.", file_path);
    Ok(())
}

fn template_exists_already(dotfiles: &[common::Dotfile],
                           templates_path: &str,
                           template_path: &str,
                           tar_path: &str)
                           -> Result<Option<usize>, error::DotfilerError> {
    for (i, dotfile) in dotfiles.iter().enumerate() {
        let existing_template_path = common::resolve_path(&dotfile.template, Some(templates_path))?;
        let existing_tar_path = common::resolve_path(&dotfile.target, Some(templates_path))?;
        if existing_template_path == template_path || existing_tar_path == tar_path {
            return Ok(Some(i));
        }
    }

    Ok(None)
}
