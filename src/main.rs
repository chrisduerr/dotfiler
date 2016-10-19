// TODO: (Optional) Add theme system
// TODO: (Optional) Add globals with theme system
// TODO: (Optional) Copy empty directories
extern crate rustc_serialize;
extern crate handlebars;
extern crate walkdir;
extern crate sqlite;
extern crate toml;

mod stylish_templater;
mod templates;
mod utilities;
mod sqldbs;

use std::env::args;

fn main() {
    let args: Vec<_> = args().collect();
    if let Some(db_path) = args.get(2) {
        if args.get(1) == Some(&String::from("--templatedb")) {
            stylish_templater::template_db(db_path);
        }
    } else {
        if args.len() == 1 || args[1] == "--templates" {
            templates::load();
        }
        if args.len() == 1 || args[1] == "--sqldbs" {
            sqldbs::load();
        }
    }
}
