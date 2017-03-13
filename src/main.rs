#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate rusqlite;
extern crate walkdir;
extern crate clap;
extern crate toml;

mod add_template;
mod templates;
mod common;
mod error;

// TODO: Don't unwrap
// TODO: Better errors for users
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
        .subcommand(clap::SubCommand::with_name("add")
            .about("Add new directories, symlinks or files to your dotfiles.")
            .version("0.1.0")
            .author("Christian Dürr <contact@christianduerr>")
            .arg(clap::Arg::with_name("file")
                .help("File, directory or symlink you want to add.")
                .required(true)
                .index(1))
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
        let config_path = if let Some(config_path) = args.value_of("config") {
            config_path
        } else {
            "./config.toml"
        };

        let templating_enabled = !args.is_present("no-templating");
        let file = args.value_of("file").unwrap(); // Safe because required
        add_template::add_template(config_path, file, templating_enabled).unwrap();
    } else {
        let config_path = if let Some(config_path) = args.value_of("config") {
            config_path
        } else {
            "./config.toml"
        };

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
