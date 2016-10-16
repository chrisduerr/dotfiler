use walkdir::WalkDir;
use std::os::unix::fs::symlink;
use std::fs::{remove_file, copy, symlink_metadata, read_link};
use utilities;

pub fn load() {
    let dotfiles = match utilities::load_from_toml("dotfiles") {
        Some(t) => t,
        None => return,
    };

    for (dotfile, tar_path) in dotfiles {
        let tar_path = match utilities::path_value_to_string(&tar_path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &dotfile,
                         tar_path);
                continue;
            }
        };

        let src_path = format!("{}/dotfiles/{}", utilities::get_app_dir(), &dotfile);
        let src_len = src_path.len();

        for entry in WalkDir::new(&src_path) {
            let ent_src_path = entry.unwrap(); // No idea when this throws an error
            let ent_src_path = ent_src_path.path().to_string_lossy().into_owned();
            let ent_tar_path = format!("{}{}", tar_path, &ent_src_path[src_len..]);

            let src_meta = match symlink_metadata(&ent_src_path) {
                Ok(meta) => meta,
                Err(_) => {
                    println!("Faild copying {}: You don't have permission to acces {} or it \
                              doesn't exist",
                             &dotfile,
                             &ent_src_path);
                    continue;
                }
            };
            if src_meta.is_file() || src_meta.file_type().is_symlink() {
                let create_dirs_success = utilities::create_directories_for_file(&ent_tar_path);
                if !create_dirs_success {
                    println!("Failed copying {}: Could not create one or more directories \
                              required.",
                             &dotfile);
                }
            }
            if src_meta.is_file() {
                if copy(&ent_src_path, &ent_tar_path).is_err() {
                    println!("Failed copying {}: Could not copy {} to {}.",
                             &dotfile,
                             &ent_src_path,
                             &ent_tar_path);
                    continue;
                }
            } else if src_meta.file_type().is_symlink() {
                let _ = remove_file(&ent_tar_path);
                let real_path = match read_link(&ent_src_path) {
                    Ok(rp) => rp,
                    Err(_) => {
                        println!("Failed copying {}: Could not resolve Symlink path.",
                                 &dotfile);
                        continue;
                    }
                };
                if symlink(real_path, &ent_tar_path).is_err() {
                    println!("Failed copying {}: Could not create symlink.", &dotfile);
                }
            }
        }
    }
}
