use tera::{Tera, Context};
use toml::Table;
use std::fs::File;
use std::io::Write;

pub fn load(home_dir: &str, app_dir: &str, config: &Table) {
    let variables = config
        .get("variables")
        .expect("No [variables] section found")
        .as_table().expect("[variables] is not a valid TOML table");
    let mut context = Context::new();

    for (key, val) in variables {
        context.add(key, &val.as_str().expect("error: value not a valid string"));
    };

    let tera = Tera::new(format!("{}/templates/*", app_dir).as_str());

    let templates = config
        .get("templates")
        .expect("No [templates] section found")
        .as_table().expect("[templates] is not a valid TOML table");

    for (template, path) in templates {
        let path_str = path.as_str().expect("error: path not a valid string");
        let path = path_str.replace("$HOME", home_dir).replace("~", home_dir);
        let render = tera
            .render(template, context.clone())
            .expect("Could not find template!");
        let mut file = File::create(&path)
            .expect(format!("Couldn't access {}", path).as_str());
        let _ = file.write_all(render.as_bytes());
    };
}
