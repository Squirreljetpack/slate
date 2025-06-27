use anyhow::{bail, Ok, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, fs, io::{stdin, Read}, path::PathBuf, process::Command
};
use log::{error,info};


pub type Section = IndexMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)] // Allows UnitFile to be treated as IndexMap for serde
pub struct UnitFile(pub IndexMap<String, Section>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SystemdUnits(pub HashMap<String, UnitFile>);

extern "C" {
    fn geteuid() -> u32;
}

fn systemctl_cmd(is_root: bool) -> Command {
    let mut cmd = Command::new("systemctl");
    if !is_root {
        cmd.arg("--user");
    }
    cmd
}

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
        info!("Delete the failed files? [y/N]");
        let mut del_buffer = [0; 1];
        stdin().read_exact(&mut del_buffer)?;

        if del_buffer[0] == b'y' || del_buffer[0] == b'Y' {
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
        
    println!("Activate the new service files? (Ensure your files have been created in the correct directories!) [y/N]: ");
    let mut buffer = [0; 1];
    stdin().read_exact(&mut buffer)?;

    if buffer[0] == b'y' || buffer[0] == b'Y' {
        
        let is_root = unsafe { geteuid() == 0 };

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

pub fn process_systemd_configs(configs: SystemdUnits) -> Result<SystemdUnits> {
    let mut output_units: HashMap<String, UnitFile> = HashMap::new();

    for (unit_name, unit_file_struct) in configs.0 {
        let mut unit = unit_file_struct.0;

        let mut processed_unit = IndexMap::new();
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

        let service_section = processed_unit
            .entry("Service".to_string())
            .or_insert_with(IndexMap::new);

        if timer_section_content.is_some() {
            service_section
                .entry("Type".to_string())
                .or_insert_with(|| "oneshot".to_string());
        }

        service_section.insert("StandardOutput".to_string(), "journal".to_string());
        service_section.insert("StandardError".to_string(), "journal".to_string());

        let service_filename = format!("{}.service", unit_name);
        output_units.insert(service_filename, UnitFile(processed_unit));

        // Create a seperate Unit for the Timer section
        if let Some(timer_content) = timer_section_content {
            let mut timer_unit = IndexMap::new();

            let mut timer_unit_unit = IndexMap::new();
            let mut timer_unit_timer = IndexMap::new();
            let mut timer_unit_install = IndexMap::new();

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
            output_units.insert(timer_filename, UnitFile(timer_unit));
        }
    }
    Ok(SystemdUnits(output_units))
}


