use rustc_serialize::json::Json;
use handlebars::Handlebars;
use sqlite::{State, Connection, open};
use std::fs::copy;
use utilities;

pub fn load() {
    let variables = match utilities::get_variables_json() {
        Some(c) => c,
        None => return,
    };

    let sqldbs = match utilities::load_from_toml("sqldbs") {
        Some(t) => t,
        None => return,
    };

    for (sqldb, tar_path) in sqldbs {
        let src_path = format!("{}/sqldbs/{}", utilities::get_app_dir(), &sqldb);
        let tar_path = match utilities::path_value_to_string(&tar_path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &sqldb,
                         &tar_path);
                continue;
            }
        };

        let create_dirs_success = utilities::create_directories_for_file(&tar_path);
        if !create_dirs_success {
            println!("Failed copying {}: Could not create one or more directories required.",
                     &sqldb);
            continue;
        }

        let copy_success = copy(&src_path, &tar_path).is_ok();
        if !copy_success {
            println!("Failed copying {}: Could not copy file.", &sqldb);
            continue;
        }


        let render_success = render_db(&tar_path, &variables);
        if !render_success {
            println!("Failed copying {}: Unable to render DB.", &sqldb);
            continue;
        }
    }
}

fn render_db(db_path: &str, template_data: &Json) -> bool {
    let db_connection = match open(&db_path) {
        Ok(db) => db,
        Err(_) => {
            println!("Could not find {}.", db_path);
            return false;
        }
    };

    let tables = get_vec_from_db(&db_connection,
                                 "SELECT tbl_name FROM sqlite_master WHERE type = 'table';",
                                 0);

    for table in tables {
        let columns = get_vec_from_db(&db_connection, &format!("PRAGMA table_info({});", table), 1);

        for column in columns {
            let current_entries = get_vec_from_db(&db_connection,
                                                  &format!("SELECT {} FROM {};", column, table),
                                                  0);

            for current_entry in current_entries {
                let current_entry = current_entry.replace("'", "''");

                let mut new_entry = current_entry.clone();
                let handlebars = Handlebars::new();
                new_entry = match handlebars.template_render(&new_entry, template_data) {
                    Ok(rendered_str) => rendered_str,
                    Err(_) => {
                        println!("Error while rendering DB: Could not render {}",
                                 current_entry);
                        continue;
                    }
                };

                db_connection.execute(&format!("UPDATE {} SET {} = '{}' WHERE {} = '{}';",
                                      table,
                                      column,
                                      new_entry,
                                      column,
                                      current_entry))
                    .unwrap();
            }
        }
    }
    true
}

fn get_vec_from_db(db_conn: &Connection, query: &str, index: usize) -> Vec<String> {
    let mut tables = Vec::new();
    let mut statement = match db_conn.prepare(query) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    while let State::Row = match statement.next() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    } {
        match statement.read::<String>(index) {
            Ok(s) => tables.push(s),
            Err(_) => continue,
        };
    }

    tables
}
