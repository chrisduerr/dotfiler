#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate rusqlite;
extern crate walkdir;
extern crate clap;
extern crate toml;

use std::{io, fs, path};

mod template_creator;
mod templates;
mod common;
mod error;

// TODO: Don't unwrap
// TODO: Better errors
// TODO: templates_dir is used three times and too complex
// TODO: Add "add" command for adding stuff without having to modify config
fn main() {
    let args = clap::App::new("Dotfiler")
        .version("0.1.0")
        .author("Christian Dürr <contact@christianduerr.com>")
        .about("A powerful dotfile manager")
        .arg(clap::Arg::with_name("no-files")
            .long("no-files")
            .conflicts_with("no-sqlite")
            .help("Don't copy normal files to their location."))
        .arg(clap::Arg::with_name("no-sqlite")
            .long("sqlite")
            .conflicts_with("no-files")
            .help("Don't copy SQLite files to their location."))
        .arg(clap::Arg::with_name("dry")
            .short("d")
            .long("dry")
            .help("Copy the files to the './dry/' directory instead of replacing the originals."))
        .arg(clap::Arg::with_name("config")
            .short("c")
            .long("config")
            .help("An alternative location for the config file. The default is './config.toml'")
            .value_name("FILE"))
        .arg(clap::Arg::with_name("create-templates")
            .long("create-templates")
            .help("Load the current files as templates without applying new config changes.")
            .conflicts_with_all(&["no-files", "no-sqlite", "dry"]))
        .get_matches();

    let config_path = if let Some(config_path) = args.value_of("config") {
        config_path
    } else {
        "./config.toml"
    };

    // Create templates dir if not existing
    let templates_dir = path::Path::new(config_path)
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))
        .unwrap()
        .join("templates");
    fs::create_dir_all(templates_dir).unwrap();

    // Create templated files from "old.toml"
    template_creator::create_templates(config_path).unwrap();

    if !args.is_present("create-templates") {
        let root_path = if args.is_present("dry") {
            [&templates::get_working_dir().unwrap(), "dry/"].concat()
        } else {
            String::from("/")
        };

        let copy_files = !args.is_present("no-files");
        let copy_sqlite = !args.is_present("no-sqlite");

        templates::load(&root_path, config_path, copy_files, copy_sqlite).unwrap();
    }
}
