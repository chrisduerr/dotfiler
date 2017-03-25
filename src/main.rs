#[macro_use]
extern crate serde_derive;
extern crate handlebars;
extern crate tempfile;
extern crate rusqlite;
extern crate walkdir;
extern crate clap;
extern crate toml;

// mod add_template;
mod filesystem;
mod templates;
mod common;
mod error;

// TODO: Better errors for users
// TODO: No logging required Errors should never reach main.rs (so no unwrap required)
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
        println!("This functionality hasn't been added yet. \
                 You can look forward to it in the near future!");
        // Safe because "file" is required
        // let file = args.value_of("file").unwrap();
        // let config_path = get_config_dir(args.value_of("config"));
        // let templating_enabled = !args.is_present("no-templating");
        // let new_name = args.value_of("name");

        // add_template::add_template(&config_path, file, new_name, templating_enabled).unwrap();
    } else {
        let config_path = get_config_dir(args.value_of("config"));
        let root_path = if args.is_present("dry") {
            [&common::get_working_dir().unwrap(), "/dry/"].concat()
        } else {
            String::from("/")
        };

        templates::load(&root_path, &config_path).unwrap();
    }
}

fn get_config_dir(config: Option<&str>) -> String {
    match config {
        Some(config) => config.to_string(),
        None => String::from("./config.toml"),
    }
}
