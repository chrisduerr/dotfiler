use tera::Tera;
use std::fs::File;
use std::io::Write;
use utilities;

pub fn load() {
    let variables_context = match utilities::get_variables_context() {
        Some(c) => c,
        None => return,
    };

    let templates = match utilities::load_from_toml("templates") {
        Some(t) => t,
        None => return,
    };


    let tera = Tera::new(format!("{}/templates/*", utilities::get_app_dir()).as_str());
    for (template, path) in templates {
        let path = match utilities::path_value_to_string(&path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &template,
                         &path);
                continue;
            }
        };

        let render = match tera.render(&template, variables_context.clone()) {
            Ok(r) => r,
            Err(_) => {
                println!("Failed copying {}: Unable to convert template.", &template);
                continue;
            }
        };

        let create_dirs_success = utilities::create_directories_for_file(&path);
        if !create_dirs_success {
            println!("Failed copying {}: Could not create one or more directories required.",
                     &template);

        }

        let mut file = match File::create(&path) {
            Ok(f) => f,
            Err(_) => {
                println!("Failed copying {}: Could not create target file.",
                         &template);
                continue;
            }
        };
        let _ = file.write_all(render.as_bytes());
    }
}
