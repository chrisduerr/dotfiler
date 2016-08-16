// TODO: Fix Symlinks
// TODO: Add theme system
extern crate walkdir;
extern crate tera;
extern crate toml;

mod templates;
mod dotfiles;

use toml::Parser;
use std::fs::File;
use std::io::Read;
use std::process::exit;
use std::env::{current_exe, home_dir, args};

fn main() {
    let home_dir = home_dir().unwrap();
    let home_dir = home_dir.to_str().unwrap();
    let mut app_dir = current_exe().unwrap();
    app_dir.pop();
    let app_dir = app_dir.to_str().unwrap();

    let mut buffer = String::new();
    let _ = File::open(format!("{}/config.toml", app_dir).as_str())
        .expect("Couldn't find configuration file")
        .read_to_string(&mut buffer);

    let config = match Parser::new(&buffer).parse() {
        Some(config) => config,
        None => {
            println!("error: could not parse configuration file");
            exit(1);
        }
    };

    let args: Vec<_> = args().collect();
    if args.len() == 1 || args[1] == "--templates" {
        templates::load(home_dir, app_dir, &config);
    }
    if args.len() == 1 || args[1] == "--dotfiles" {
        dotfiles::load(home_dir, app_dir, &config);
    }
}
