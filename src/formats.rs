use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use std::collections::HashMap;

pub type Section = IndexMap<String, String>;
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)] // Allows UnitFile to be treated as IndexMap for serde
pub struct Ini(pub IndexMap<String, Section>);

impl Ini {
    pub fn new() -> Self {
        Ini(IndexMap::new())
    }

    pub fn insert(&mut self, key: String, value: Section) -> Option<Section> {
        self.0.insert(key, value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IniFiles(pub HashMap<String, Ini>);

impl IniFiles {
    pub fn new() -> Self {
        IniFiles(HashMap::new())
    }

    pub fn insert(&mut self, key: String, value: Ini) -> Option<Ini> {
        self.0.insert(key, value)
    }
}
