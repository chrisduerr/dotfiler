use toml::{Table, Value};
use walkdir::WalkDir;
use std::path::Path;
use std::os::unix::fs::symlink;
use std::fs::{remove_file, create_dir_all, copy, symlink_metadata, read_link};

pub fn load(home_dir: &str, app_dir: &str, config: &Table) {
    let dotfiles = match config.get("dotfiles").and_then(Value::as_table) {
        Some(t) => t,
        None => {
            println!("[dotfiles] section is missing or invalid.");
            return;
        }
    };

    let mut failed = Vec::new();
    for (dotfile, tar_path) in dotfiles {
        let tar_path = match tar_path.as_str() {
            Some(s) => s,
            None => {
                failed.push((dotfile, String::from("Value is not a valid string.")));
                continue;
            },
        };
        let tar_path = tar_path.replace("$HOME", home_dir).replace("~", home_dir);
        let src_path = format!("{}/dotfiles/{}", app_dir, dotfile);
        let src_len = src_path.len();

        for entry in WalkDir::new(&src_path) {
            let ent_src_path = entry.unwrap(); // No idea when this throws an error
            let ent_src_path = ent_src_path.path().to_string_lossy().into_owned();
            let ent_tar_path = format!("{}{}", tar_path, &ent_src_path[src_len..]);

            let src_meta = match symlink_metadata(&ent_src_path) {
                Ok(meta) => meta,
                Err(_) => {
                    failed.push((dotfile, String::from("You don't have permissions \
                                 to access {} or it doesn't exist.")));
                    continue;
                },
            };
            if src_meta.is_file() || src_meta.file_type().is_symlink() {
                if create_dir_all(match Path::new(&ent_tar_path).parent() {
                    Some(parent_path) => parent_path,
                    None => {
                        failed.push((dotfile, String::from("Target directory can't \
                                     be root.")));
                        continue;
                    },
                }).is_err() {
                    failed.push((dotfile, String::from("Could not create one or \
                                 more directories required.")));
                    continue;
                }
            }
            if src_meta.is_file() {
                if copy(&ent_src_path, &ent_tar_path).is_err() {
                    failed.push((dotfile, format!("Could not copy {} to {}.",
                                                  &ent_src_path, &ent_tar_path)));
                    continue;
                }
            }
            else if src_meta.file_type().is_symlink() {
                let _ = remove_file(&ent_tar_path);
                let real_path = match read_link(&ent_src_path) {
                    Ok(rp) => rp,
                    Err(_) => {
                        failed.push((dotfile, String::from("Could not resolve the \
                                     symlink's path.")));
                        continue;
                    },
                };
                if symlink(real_path, &ent_tar_path).is_err() {
                    failed.push((dotfile, String::from("Could not create symlink.")));
                }
            }
        }
    }

    for failure in failed {
        println!("Failed copying {}: {}", failure.0, failure.1);
    }
}
