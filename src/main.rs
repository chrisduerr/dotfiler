#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate rusqlite;
extern crate walkdir;
extern crate clap;
extern crate toml;

mod templates;
mod error;

// TODO: Don't unwrap
fn main() {
    let args = clap::App::new("Dotfiler")
        .version("0.1.0")
        .author("Christian Dürr <contact@christianduerr.com>")
        .about("A powerful dotfile manager")
        .subcommand(clap::SubCommand::with_name("templatedb")
            .version("0.1.0")
            .author("Christian Dürr <contact@christianduerr.com>")
            .about("Used to replace config.toml variables in SQLite DBs with template \
                    placeholders.")
            .arg(clap::Arg::with_name("FILE")
                .help("The SQLite DBs that should be templated.")
                .required(true)
                .multiple(true)))
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
        .get_matches();

    if let Some(args) = args.subcommand_matches("templatedb") {
        if let Some(_files) = args.values_of("FILE") {
            unimplemented!();
        }
    } else {
        let root_path = if args.is_present("dry") {
            [&templates::get_working_dir().unwrap(), "dry/"].concat()
        } else {
            String::from("/")
        };

        let config_path = if let Some(config_path) = args.value_of("config") {
            config_path
        } else {
            "./config.toml"
        };

        let copy_files = !args.is_present("no-files");
        let copy_sqlite = !args.is_present("no-sqlite");

        templates::load(&root_path, &config_path, copy_files, copy_sqlite).unwrap();
    }
}
