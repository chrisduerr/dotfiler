use std::{fs, path};

use filesystem;
use common;
use error;

pub fn load(target_path: &str, config_path: &str) -> Result<(), error::DotfilerError> {
    let config = common::load_config(config_path)?;
    let templates_path = common::get_templates_path(config_path)?.to_string_lossy().to_string();

    if let Some(ref dotfiles) = config.dotfiles {
        for dotfile in dotfiles {
            let template_path = common::resolve_path(&dotfile.template, Some(&templates_path))?;
            let tar_path = [target_path, &common::resolve_path(&dotfile.target, None)?[1..]]
                .concat();

            // Create all required target directories before root
            let _ = path::Path::new(&tar_path).parent().map(|p| fs::create_dir_all(&p));

            let mut root = match filesystem::create_tree_from_path(&template_path, &tar_path) {
                Ok(root) => root,
                Err(e) => {
                    println!("Can't create tree for template '{}':\n{}", template_path, e);
                    continue;
                }
            };

            if let Some(ref vars) = config.variables {
                if let Err(e) = root.render(vars) {
                    println!("Unable to template the template '{}':\n{}",
                             template_path,
                             e);
                    continue;
                }
            }

            if let Err(e) = root.save() {
                println!("Unable to save the template '{}':\n{}", template_path, e);

                if let Err(e) = root.restore() {
                    println!("Critical Error! Unable to recover from failure.\n{}", e);
                }

                continue;
            }
        }
    }

    println!("Every file as been templated.");
    Ok(())
}



// -------------
//     TESTS
// -------------

#[test]
fn load_correctly_saving_example_to_dummy_dir() {
    load("./example/",
         "/home/undeadleech/Programming/Rust/dotfiler/examples/config.toml")
            .unwrap();

    let file1_ok = fs::metadata("./example/home/undeadleech/testing/Xresources").is_ok();
    let file2_ok = fs::metadata("./example/home/undeadleech/testing/Scripts").is_ok();
    let file3_ok = fs::metadata("./example/home/undeadleech/testing/config").is_ok();
    let file4_ok = fs::metadata("./example/home/undeadleech/testing/db.sqlite").is_ok();

    let _ = fs::remove_dir_all("./example/");

    assert_eq!(file1_ok, true);
    assert_eq!(file2_ok, true);
    assert_eq!(file3_ok, true);
    assert_eq!(file4_ok, true);
}

#[test]
fn load_copying_symlinks_not_target() {
    load("./symlink/",
         "/home/undeadleech/Programming/Rust/dotfiler/examples/config.toml")
            .unwrap();

    let is_symlink = fs::symlink_metadata("./symlink/home/undeadleech/testing/config")
        .unwrap()
        .file_type()
        .is_symlink();

    let _ = fs::remove_dir_all("./symlink/");

    assert_eq!(is_symlink, true);
}
