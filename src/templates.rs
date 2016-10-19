use rustc_serialize::json::Json;
use handlebars::Handlebars;
use walkdir::WalkDir;
use std::fs::{File, copy, create_dir_all, remove_file, symlink_metadata, read_link};
use std::os::unix::fs::symlink;
use std::io::{Read, Write};
use std::path::Path;
use utilities;

pub fn load() {
    let variables = match utilities::get_variables_json() {
        Some(c) => c,
        None => {
            println!("Unable to load variables from config.");
            return;
        }
    };

    let templates = match utilities::load_from_toml("templates") {
        Some(t) => t,
        None => {
            println!("Unable to load templates from config.");
            return;
        }
    };

    for (template, tar_path) in templates {
        let src_path = format!("{}/templates/{}", utilities::get_app_dir(), &template);
        let tar_path = match utilities::path_value_to_string(&tar_path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &template,
                         &tar_path);
                continue;
            }
        };

        copy_files(&src_path, &tar_path, &variables);
    }
}

fn copy_files(src_path: &str, tar_path: &str, variables: &Json) {
    for file in WalkDir::new(src_path) {
        let file_path = file.unwrap().path().to_string_lossy().into_owned();
        let file_tar_path = format!("{}{}", tar_path, &file_path[src_path.len()..]);

        let file_meta = match symlink_metadata(&file_path) {
            Ok(meta) => meta,
            Err(_) => {
                println!("Failed copying {}: Unable to obtain metadata.", &file_path);
                continue;
            }
        };

        if file_meta.is_file() || file_meta.file_type().is_symlink() {
            if let Err(e) = create_directories_for_file(&file_tar_path) {
                println!("Failed copying {}: {}", &file_path, &e);
                continue;
            }
        }

        if file_meta.is_file() {
            if let Err(e) = template_file(&file_path, &file_tar_path, variables) {
                println!("Failed copying {}: {}", &file_path, &e);
                continue;
            }
        } else if file_meta.file_type().is_symlink() {
            let _ = remove_file(&file_tar_path); // Can't overwrite symlink
            let sym_tar_path = match read_link(&file_path) {
                Ok(path) => path,
                Err(e) => {
                    println!("Failed copying {}: {}", &file_path, &e);
                    continue;
                }
            };
            if let Err(e) = symlink(&sym_tar_path, &file_tar_path) {
                println!("Failed copying {}: {}", &file_path, &e);
                continue;
            }
        }
    }
}

fn template_file(src_path: &str, tar_path: &str, variables: &Json) -> Result<(), String> {
    let mut file_content = String::new();
    let mut f = try!(File::open(src_path).map_err(|e| e.to_string()));
    match f.read_to_string(&mut file_content) {
        Ok(s) => s,
        Err(_) => {
            // Try copying normall when file is not a text file
            try!(copy(src_path, tar_path).map_err(|e| e.to_string()));
            return Ok(());
        }
    };

    let handlebars = Handlebars::new();
    let rendered = try!(handlebars.template_render(&file_content, variables)
        .map_err(|e| e.to_string()));

    let mut f = try!(File::create(&tar_path).map_err(|e| e.to_string()));
    try!(f.write_all(rendered.as_bytes()).map_err(|e| e.to_string()));
    Ok(())
}

fn create_directories_for_file(file_path: &str) -> Result<(), String> {
    let _ = match Path::new(&file_path).parent() {
        Some(p) => create_dir_all(&p),
        None => return Err(String::from("Could not find the file's parent directory.")),
    };
    Ok(())
}
