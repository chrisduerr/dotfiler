// TODO: (Optional) Add theme system
// TODO: (Optional) Add globals with theme system
extern crate walkdir;
extern crate sqlite;
extern crate tera;
extern crate toml;

mod templates;
mod utilities;
mod dotfiles;
mod sqldbs;

use std::env::args;

fn main() {
    let args: Vec<_> = args().collect();
    if args.len() == 1 || args[1] == "--templates" {
        templates::load();
    }
    if args.len() == 1 || args[1] == "--dotfiles" {
        dotfiles::load();
    }
}
