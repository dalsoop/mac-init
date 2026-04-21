//! Domain registry data — typed access via DomainId.
//!
//! Scaffolded for future migration from raw Vec<String>/Vec<Option<DomainSpec>>
//! to a single DomainStore. Currently unused — App still exposes Vec fields for
//! backward compatibility with tests.

#![allow(dead_code)]

use super::types::DomainId;
use crate::spec::DomainSpec;

pub struct DomainEntry {
    pub name: String,
    pub spec: Option<DomainSpec>,
}

pub struct DomainStore {
    entries: Vec<DomainEntry>,
}

impl DomainStore {
    pub fn new(names: Vec<String>, specs: Vec<Option<DomainSpec>>) -> Self {
        assert_eq!(names.len(), specs.len());
        let entries = names
            .into_iter()
            .zip(specs)
            .map(|(name, spec)| DomainEntry { name, spec })
            .collect();
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, id: DomainId) -> &DomainEntry {
        &self.entries[id.0]
    }

    pub fn get_mut(&mut self, id: DomainId) -> &mut DomainEntry {
        &mut self.entries[id.0]
    }

    pub fn name(&self, id: DomainId) -> &str {
        &self.entries[id.0].name
    }

    pub fn spec(&self, id: DomainId) -> Option<&DomainSpec> {
        self.entries[id.0].spec.as_ref()
    }

    pub fn set_spec(&mut self, id: DomainId, spec: Option<DomainSpec>) {
        self.entries[id.0].spec = spec;
    }

    pub fn iter(&self) -> impl Iterator<Item = (DomainId, &DomainEntry)> {
        self.entries
            .iter()
            .enumerate()
            .map(|(i, e)| (DomainId(i), e))
    }

    pub fn find_by_name(&self, name: &str) -> Option<DomainId> {
        self.entries
            .iter()
            .position(|e| e.name == name)
            .map(DomainId)
    }

    /// Expose names as a Vec<String> for backward compatibility.
    pub fn names(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    /// Expose specs as Vec<Option<DomainSpec>> for backward compatibility.
    pub fn specs(&self) -> Vec<Option<DomainSpec>> {
        self.entries.iter().map(|e| e.spec.clone()).collect()
    }
}
