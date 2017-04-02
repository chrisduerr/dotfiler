use std::{process, io, path};

use common;
use error;

pub fn execute(config_path: &str) -> Result<(), error::DotfilerError> {
    let config = common::load_config(config_path)?;
    let scripts_path = get_scripts_path(config_path)?.to_string_lossy().to_string();

    if let Some(ref scripts) = config.scripts {
        for script in scripts {
            let script: String = match common::resolve_path(&script, Some(&scripts_path)) {
                Ok(path) => path,
                Err(e) => {
                    println!("Unable to load script '{}':\n{}", script, e);
                    continue;
                }
            };

            match process::Command::new("sh").args(&["-c", &script]).output() {
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

                    if !stderr.is_empty() {
                        println!("Unable to execute script:\n{}", stderr);
                    } else if !stdout.is_empty() {
                        println!("Output for '{}':\n{}", script, stdout);
                    }
                }
                Err(e) => println!("Unable to execute '{}':\n{}", script, e),
            };
        }
    }

    println!("All scripts executed.");
    Ok(())
}

fn get_scripts_path(config_path: &str) -> Result<path::PathBuf, io::Error> {
    let config_path = common::resolve_path(config_path, None)?;
    Ok(path::Path::new(&config_path).parent().unwrap().join("scripts"))
}
