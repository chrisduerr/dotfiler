use std::io::{self, Read, Write};
use std::{fs, path};
use toml::value;

use common;
use error;

// TODO: Test
pub fn create_templates(config_path: &str) -> Result<(), error::DotfilerError> {
    let templates_dir = path::Path::new(config_path)
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Config can't be root."))?
        .join("templates");

    let old_path = templates_dir.join("old.toml").to_string_lossy().to_string();
    let config = match common::load_config(&old_path) {
        Ok(conf) => conf,
        Err(_) => common::load_config(config_path)?,
    };

    if let Some(ref templates) = config.templates {
        for template in templates {
            let src_str = templates_dir.join(&template.template).to_string_lossy().to_string();
            let template_path = common::resolve_path(&src_str)?;
            let tar_path = &common::resolve_path(&template.target)?;

            if common::is_sqlite(tar_path)? {
                create_sqlite_template(&template_path, tar_path, &config.variables)?;
            } else {
                create_file_template(&template_path, tar_path, &config.variables)?;
            }
        }
    }

    Ok(())
}

pub fn create_file_template(template_path: &str,
                            tar_path: &str,
                            variables: &value::Table)
                            -> Result<(), error::DotfilerError> {
    let mut content = String::new();
    fs::File::open(tar_path)?.read_to_string(&mut content)?;

    for (key, val) in variables {
        let val_str = val.as_str()
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput,
                               format!("Variable \"{}\" is not a String.", key))
            })?;
        content = content.replace(val_str, &format!("{{{{ {} }}}}", key));
    }

    Ok(fs::File::create(template_path)?.write_all(content.as_bytes())?)
}

pub fn create_sqlite_template(template_path: &str,
                              tar_path: &str,
                              variables: &value::Table)
                              -> Result<(), error::DotfilerError> {
    fs::copy(tar_path, template_path)?;

    fn modify(entry: &str, variables: &value::Table) -> Result<String, error::DotfilerError> {
        let mut new_entry = entry.to_owned();
        for (key, val) in variables {
            if let Some(val) = val.as_str() {
                new_entry = new_entry.replace(&val, &format!("{{{{ {} }}}}", key));
            }
        }
        Ok(new_entry)
    };

    common::modify_sqlite_elements(template_path, modify, variables)
}



// -------------
//     TESTS
// -------------

#[cfg(test)]
use std::collections::BTreeMap;

#[test]
fn create_file_template_correctly_creating_file_template() {
    let tar_path = "create_file_template_correctly_creating_file_template_input";
    let template_path = "create_file_template_correctly_creating_file_template_output";

    fs::File::create(tar_path)
        .unwrap()
        .write_all("test: #123456".as_bytes())
        .unwrap();

    let mut vars = BTreeMap::new();
    vars.insert(String::from("test"),
                value::Value::String(String::from("#123456")));

    create_file_template(template_path, tar_path, &vars).unwrap();

    let mut output = String::new();
    fs::File::open(template_path).unwrap().read_to_string(&mut output).unwrap();

    let _ = fs::remove_file(tar_path);
    let _ = fs::remove_file(template_path);

    assert_eq!(output, "test: {{ test }}");
}
