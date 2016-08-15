use toml::Table;
use walkdir::WalkDir;
use std::path::Path;
use std::fs::{create_dir_all, copy, metadata};

pub fn load(home_dir: &str, app_dir: &str, config: &Table) {
    let dotfiles = config
        .get("dotfiles")
        .expect("No [dotfiles] section found")
        .as_table().expect("[templates] is not a valid TOML table");

    for (dotfile, tar_path) in dotfiles {
        let tar_path = tar_path.as_str().expect("error: path not a valid string");
        let tar_path = tar_path.replace("$HOME", home_dir).replace("~", home_dir);
        let src_path = format!("{}/dotfiles/{}", app_dir, dotfile);
        let src_len = src_path.len();

        for entry in WalkDir::new(&src_path) {
            let ent_src_path = entry
                .expect("There was an error accessing the dotfile source.");
            let ent_src_path = ent_src_path.path().to_str()
                .expect("Path is not a valid string.");
            let ent_tar_path = format!("{}{}", tar_path, &ent_src_path[src_len..]);

            let src_meta = metadata(&ent_src_path)
                .expect("Could not get file metadata.");
            if src_meta.is_file() {
                create_dir_all(Path::new(&ent_tar_path).parent()
                               .expect("Invalid path structure."))
                    .expect("Could not create target directory.");

                copy(&ent_src_path, &ent_tar_path)
                    .expect(format!("Could not copy {}.", &ent_src_path).as_str());

                println!("Copied {} to {}", ent_src_path, ent_tar_path);
            }
        }
    }
}
