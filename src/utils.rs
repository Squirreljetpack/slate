use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::{collections::HashMap, fs, path::PathBuf};

pub fn to_ini_string<T: Serialize>(value: &T) -> Result<String> {
    let json_value = serde_json::to_value(value)?;
    let map = match json_value {
        Value::Object(map) => map,
        _ => {
            return Err(anyhow!(
                "INI serializer expects a map-like structure at the top level"
            ))
        }
    };

    let mut result = String::new();
    for (section_name, section_value) in map {
        result.push_str(&format!("[{}]\n", section_name));
        let section_map = match section_value {
            Value::Object(map) => map,
            _ => return Err(anyhow!("INI section '{}' is not a map.", section_name)),
        };

        for (key, val) in section_map {
            let val_str = match val {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => {
                    return Err(anyhow!(
                        "Value for key '{}' in section '{}' is not a string, number, or boolean.",
                        key,
                        section_name
                    ))
                }
            };
            result.push_str(&format!("{}={}\n", key, val_str));
        }
        result.push('\n');
    }

    if result.ends_with("\n\n") {
        result.pop();
        result.pop();
    } else if result.ends_with('\n') {
        result.pop();
    }

    Ok(result)
}

pub fn write_files<T, S>(
    units: HashMap<String, T>,
    output_dir: &PathBuf,
    serializer: S,
) -> Result<Vec<PathBuf>>
where
    T: Serialize,
    S: Fn(&T) -> Result<String>,
{
    let mut written_files = Vec::new();
    for (filename, unit) in units {
        let string_content =
            serializer(&unit).with_context(|| format!("Failed to serialize unit: {}", filename))?;

        let file_path = output_dir.join(&filename);

        fs::write(&file_path, string_content)
            .with_context(|| format!("Failed to write to file: {:?}", file_path))?;
        written_files.push(file_path);
    }

    Ok(written_files)
}

pub fn print_files<T, S>(units: HashMap<String, T>, serializer: S) -> Result<()> 
where
    T: Serialize,
    S: Fn(&T) -> Result<String>,
{
    let len = units.len();
    for (i, (filename, unit)) in units.into_iter().enumerate() {
        let string_content =
            serializer(&unit).with_context(|| format!("Failed to serialize unit: {}", filename))?;

        println!("# {filename}\n{string_content}");
        if i + 1 < len {
            println!("\n---\n");
        }
    }

    Ok(())
}

pub fn is_interactive() -> bool {
    if let Ok(file) = fs::OpenOptions::new().read(true).open("/dev/tty") {
        let metadata = file.metadata().unwrap();
        let permissions = metadata.permissions();
        std::os::unix::fs::PermissionsExt::mode(&permissions) & 0o222 != 0
    } else {
        false
    }
}