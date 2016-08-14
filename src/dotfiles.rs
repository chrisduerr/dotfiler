use toml::Table;
use std::fs::File;
use std::io::{Write, Read};

pub fn load(home_dir: &str, app_dir: &str, config: &Table) {
    let dotfiles = config
        .get("dotfiles")
        .expect("No [dotfiles] section found")
        .as_table().expect("[templates] is not a valid TOML table");
    for (dotfile, path) in dotfiles {
        let path_str = path.as_str().expect("error: path not a valid string");
        let path = path_str.replace("$HOME", home_dir).replace("~", home_dir);
        let mut buffer = String::new();
        let _ = File::open(format!("{}/dotfiles/{}", app_dir, dotfile).as_str())
            .expect(format!("Couldn't find dotfile dotfiles/{}", dotfile).as_str())
            .read_to_string(&mut buffer);
        let mut file = File::create(&path)
            .expect(format!("Couldn't access {}", path).as_str());
        let _ = file.write_all(buffer.as_bytes());
    }
}
