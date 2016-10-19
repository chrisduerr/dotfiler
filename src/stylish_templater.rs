use sqlite;
use toml::Table;
use utilities;

pub fn template_db(user_input: &str) {
    let sqlite_file_path = utilities::fix_home_path(user_input);

    let connection = match sqlite::open(&sqlite_file_path) {
        Ok(o) => o,
        Err(_) => {
            println!("Unable to open {} as sqlite database.", &sqlite_file_path);
            return;
        }
    };

    let variables: Table = match utilities::load_from_toml("variables") {
        Some(s) => s,
        None => {
            println!("Could not find variables in config file.");
            return;
        }
    };

    let tables = utilities::get_vec_from_db(&connection,
                                            "SELECT tbl_name FROM sqlite_master WHERE type = \
                                             'table';",
                                            0);
    for table in tables {
        let columns =
            utilities::get_vec_from_db(&connection, &format!("PRAGMA table_info({});", &table), 1);
        for column in columns {
            let current_entries =
                utilities::get_vec_from_db(&connection,
                                           &format!("SELECT {} FROM {};", column, table),
                                           0);
            for current_entry in current_entries {
                let current_entry = current_entry.replace("'", "''");

                let mut new_entry = current_entry.clone();
                for (key, val) in &variables {
                    if let Some(val) = val.as_str() {
                        new_entry = new_entry.replace(&val, &format!("{{{{ {} }}}}", &key));
                    }
                }

                let _ = connection.execute(&format!("UPDATE {} SET {} = '{}' WHERE {} = '{}';",
                                                    &table,
                                                    &column,
                                                    &new_entry,
                                                    &column,
                                                    &current_entry));
            }
        }
    }
}
