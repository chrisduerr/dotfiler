use handlebars::Handlebars;
use std::fs::File;
use std::io::{Read, Write};
use utilities;

pub fn load() {
    let variables = match utilities::get_variables_json() {
        Some(c) => c,
        None => return,
    };

    let templates = match utilities::load_from_toml("templates") {
        Some(t) => t,
        None => return,
    };

    for (template, tar_path) in templates {
        let mut src_content = String::new();
        let src_path = format!("{}/templates/{}", utilities::get_app_dir(), &template);
        let _ = match File::open(&src_path) {
            Ok(mut f) => f.read_to_string(&mut src_content),
            Err(_) => {
                println!("Failed copying {}: Unable to read source file", &template);
                continue;
            }
        };

        let tar_path = match utilities::path_value_to_string(&tar_path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &template,
                         &tar_path);
                continue;
            }
        };

        let handlebars = Handlebars::new();
        let rendered = match handlebars.template_render(&src_content, &variables) {
            Ok(rendered_str) => rendered_str,
            Err(_) => {
                println!("Failed copying {}: Unable to render template.", &template);
                continue;
            }
        };

        let create_dirs_success = utilities::create_directories_for_file(&tar_path);
        if !create_dirs_success {
            println!("Failed copying {}: Could not create one or more directories required.",
                     &template);
            continue;
        }

        let _ = match File::create(&tar_path) {
            Ok(mut f) => f.write_all(rendered.as_bytes()),
            Err(_) => {
                println!("Failed copying {}: Could not create target file.",
                         &template);
                continue;
            }
        };
    }
}
