use anyhow::{Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap};

pub type Section = IndexMap<String, String>;

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)] // Allows UnitFile to be treated as IndexMap for serde
pub struct UnitFile(pub IndexMap<String, Section>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SystemdUnits(pub HashMap<String, UnitFile>);

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


