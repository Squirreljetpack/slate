use anyhow::{Ok, Result};
use std::{
    collections::HashMap, fs, path::PathBuf, process::Command
};
use log::{error,info};

use crate::{formats::{Ini, IniFiles, Section}, utils::{self, systemctl_cmd}};

pub fn activate_units(written_files: Vec<PathBuf>) -> anyhow::Result<()> {

    info!("Verifying systemd units");
    let mut failed_files = Vec::new();
    for file in &written_files {
        let status = Command::new("systemd-analyze")
            .arg("verify")
            .arg(file)
            .status()?;

        if !status.success() {
            error!("Verification failed for {}", file.display());
            failed_files.push(file);
        }
    }

    if !failed_files.is_empty() {
        info!("One or more unit files failed verification.");

        // Prompt to delete failed files
        if utils::ask_confirm("Delete the failed files?", false)? {
            for file in &failed_files {
                if let Err(e) = fs::remove_file(file) {
                    error!("Failed to delete {}: {}", file.display(), e);
                } else {
                    info!("Deleted {}", file.display());
                }
            }
        } else {
            info!("Failed files were not deleted.");
        }

        info!("Skipping activation due to invalid files.");
        return Ok(());
    }
    info!("All units passed!");
        
    if utils::ask_confirm("Activate the new service files? (Ensure your files have been created in the correct directories!)", true)? {
        
        let is_root = utils::is_root();

        systemctl_cmd(is_root).arg("daemon-reload").status()?;

        for file in &written_files {

            let file_name = file.file_name().unwrap().to_str().unwrap();

            if file_name.ends_with(".timer") {
                systemctl_cmd(is_root)
                    .args(["enable", "--now", file_name])
                    .status()?;
            } else if file_name.ends_with(".service") {
                let service_base = file_name.strip_suffix(".service").unwrap();
                let timer_exists = written_files.iter().any(|f| {
                    f.file_name()
                        .unwrap()
                        .to_str()
                        .map(|n| n == format!("{}.timer", service_base))
                        .unwrap_or(false)
                });

                if !timer_exists {
                    systemctl_cmd(is_root)
                        .args(["enable", "--now", file_name])
                        .status()?;
                }
            }
        }
    }

    Ok(())
}

pub fn process_systemd(configs: IniFiles) -> Result<IniFiles> {
    let mut output_units: HashMap<String, Ini> = HashMap::new();

    for (unit_name, unit_file_struct) in configs.0 {
        let mut unit = unit_file_struct.0;

        let mut processed_unit = Ini::new();
        let mut timer_section_content: Option<Section> = None;

        for (section_name, section_content) in unit.iter_mut() {
            // Timer section handled separately
            if section_name == "Timer" {
                timer_section_content = Some(section_content.clone());
                continue;
            }
            processed_unit.insert(section_name.clone(), section_content.clone());
        }

        // Insert defaults for [Service]

        let service_section = processed_unit.0
            .entry("Service".to_string())
            .or_insert_with(Section::new);

        if timer_section_content.is_some() {
            service_section
                .entry("Type".to_string())
                .or_insert_with(|| "oneshot".to_string());
        }

        service_section.insert("StandardOutput".to_string(), "journal".to_string());
        service_section.insert("StandardError".to_string(), "journal".to_string());

        let service_filename = format!("{}.service", unit_name);
        output_units.insert(service_filename, processed_unit);

        // Create a seperate Unit for the Timer section
        if let Some(timer_content) = timer_section_content {
            let mut timer_unit = Ini::new();

            let mut timer_unit_unit = Section::new();
            let mut timer_unit_timer = Section::new();
            let mut timer_unit_install = Section::new();

            for (key, value) in timer_content.iter() {
                // Handle Description seperately
                if key == "Description" {
                    timer_unit_unit.insert(key.clone(), value.clone());
                    continue;
                }
                timer_unit_timer.insert(key.clone(), value.clone());
            }

            // Insert defaults for [Unit]
            timer_unit_unit
                .entry("Description".to_string())
                .or_insert_with(|| format!("Timer for {}", unit_name));

            // Autodefine the other sections
            timer_unit_timer.insert("Unit".to_string(), format!("{}.service", unit_name));
            timer_unit_install.insert("WantedBy".to_string(), "timers.target".to_string());

            // Assemble the final timer file from its sections.
            timer_unit.insert("Unit".to_string(), timer_unit_unit);
            timer_unit.insert("Timer".to_string(), timer_unit_timer);
            timer_unit.insert("Install".to_string(), timer_unit_install);

            let timer_filename = format!("{}.timer", unit_name);
            output_units.insert(timer_filename, timer_unit);
        }
    }
    Ok(IniFiles(output_units))
}




#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{Ini, IniFiles, Section};
    use std::collections::HashMap;

    #[test]
    fn service_with_timer() {
        let mut units = HashMap::new();
        let mut unit_content = Ini::new();

        let mut unit_section = Section::new();
        unit_section.insert("Description".to_string(), "A test service".to_string());
        unit_content.insert("Unit".to_string(), unit_section);

        let mut service_section = Section::new();
        service_section.insert("ExecStart".to_string(), "/bin/echo 'Hello'".to_string());
        unit_content.insert("Service".to_string(), service_section);

        let mut timer_section = Section::new();
        timer_section.insert("OnCalendar".to_string(), "daily".to_string());
        unit_content.insert("Timer".to_string(), timer_section);

        units.insert("test".to_string(), unit_content);

        let result = process_systemd(IniFiles(units)).unwrap();

        let service = result.0.get("test.service").unwrap();
        let timer = result.0.get("test.timer").unwrap();

        insta::assert_yaml_snapshot!("service_with_timer_service", service);
        insta::assert_yaml_snapshot!("service_with_timer_timer", timer);
    }
}