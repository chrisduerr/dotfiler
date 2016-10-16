use utilities;

pub fn load() {
    let variables_context = match utilities::get_variables_context() {
        Some(c) => c,
        None => return,
    };

    let sqldbs = match utilities::load_from_toml("sqldbs") {
        Some(t) => t,
        None => return,
    };

    for (sqldb, path) in sqldbs {
        let path = match utilities::path_value_to_string(&path) {
            Some(s) => s,
            None => {
                println!("Failed copying {}: {} is not a valid path String.",
                         &sqldbs,
                         &path);
                continue;
            }
        };
    }
}
