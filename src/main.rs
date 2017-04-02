#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate tempfile;
extern crate rusqlite;
extern crate walkdir;
extern crate clap;
extern crate toml;

use std::fs;

mod add_template;
mod filesystem;
mod templates;
mod scripts;
mod common;
mod error;

// TODO: Check for diffs instead of overwriting every file
// TODO: Add ability to add script through command
fn main() {
    let args = clap::App::new("Dotfiler")
        .version("0.1.0")
        .author("Christian Dürr <contact@christianduerr.com>")
        .about("A powerful dotfile manager")
        .arg(clap::Arg::with_name("dry")
            .short("d")
            .long("dry")
            .help("Copy the files to the './dry/' directory instead of replacing the originals."))
        .arg(clap::Arg::with_name("config")
            .short("c")
            .long("config")
            .help("An alternative location for the config file. The default is './config.toml'")
            .value_name("FILE"))
        .subcommand(clap::SubCommand::with_name("add")
            .about("Add new directories, symlinks or files to your dotfiles.")
            .version("0.1.0")
            .author("Christian Dürr <contact@christianduerr>")
            .arg(clap::Arg::with_name("file")
                .help("File, directory or symlink you want to add.")
                .required(true)
                .index(1))
            .arg(clap::Arg::with_name("name")
                .short("n")
                .long("name")
                .value_name("NAME")
                .help("An alternative name for the new template file."))
            .arg(clap::Arg::with_name("config")
                .short("c")
                .long("config")
                .help("An alternative location for the config file. The default is \
                       './config.toml'")
                .value_name("FILE"))
            .arg(clap::Arg::with_name("no-templating")
                .long("no-templating")
                .help("Do not replace Strings in the files with matching variables from config.")))
        .get_matches();

    if let Some(args) = args.subcommand_matches("add") {
        let file = args.value_of("file").unwrap();
        let config_path = get_config_dir(args.value_of("config"));
        let templating_enabled = !args.is_present("no-templating");
        let new_name = args.value_of("name");

        let result = add_template::add_template(&config_path, file, new_name, templating_enabled);
        if let Err(e) = result {
            println!("{}", e);
        }
    } else {
        let config_path = get_config_dir(args.value_of("config"));
        let root_path = if args.is_present("dry") {
            [&common::get_working_dir().unwrap(), "/dry/"].concat()
        } else {
            String::from("/")
        };

        if let Err(e) = templates::load(&root_path, &config_path) {
            println!("{}", e);
        } else if let Err(e) = scripts::execute(&config_path) {
            println!("{}", e);
        }
    }
}

fn get_config_dir(config: Option<&str>) -> String {
    let config = match config {
        Some(config) => config,
        None => "config.toml",
    };

    match common::resolve_path(config, None) {
        Ok(path) => {
            match fs::metadata(&path) {
                Ok(_) => path,
                Err(e) => panic!("Unable to find config file:\n{}", e),
            }
        }
        Err(e) => panic!("Invalid config file path:\n{}", e),
    }
}
