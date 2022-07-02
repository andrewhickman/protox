use std::collections::{hash_map, HashMap};

use logos::Span;

use super::{CheckError, DefinitionKind};

/// A simple map of all definitions in a proto file for checking downstream files.
#[derive(Debug)]
pub(crate) struct NameMap {
    map: HashMap<String, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    kind: DefinitionKind,
    span: Span,
    public: bool,
    file: Option<String>,
}

impl NameMap {
    pub fn new() -> Self {
        NameMap {
            map: HashMap::new(),
        }
    }

    pub(super) fn add(
        &mut self,
        name: String,
        kind: DefinitionKind,
        span: Span,
        file: Option<&str>,
        public: bool,
    ) -> Result<(), CheckError> {
        match self.map.entry(name) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Entry {
                    file: file.map(ToOwned::to_owned),
                    kind,
                    span,
                    public,
                });
                Ok(())
            }
            hash_map::Entry::Occupied(entry) => match (kind, entry.get().kind) {
                (DefinitionKind::Package, DefinitionKind::Package) => Ok(()),
                _ => Err({
                    let name = entry.key().clone();
                    if let Some(first_file) = &entry.get().file {
                        if let Some(second_file) = file {
                            CheckError::DuplicateNameInImports {
                                name,
                                first_file: first_file.clone(),
                                second_file: second_file.to_owned(),
                            }
                        } else {
                            CheckError::DuplicateNameInFileAndImport {
                                name,
                                first_file: first_file.clone(),
                                second: span,
                            }
                        }
                    } else {
                        CheckError::DuplicateNameInFile {
                            name,
                            first: entry.get().span.clone(),
                            second: span,
                        }
                    }
                }),
            },
        }
    }

    pub(super) fn merge(
        &mut self,
        other: &Self,
        file: String,
        public: bool,
    ) -> Result<(), CheckError> {
        for (name, entry) in &other.map {
            if entry.public {
                self.add(
                    name.clone(),
                    entry.kind,
                    entry.span.clone(),
                    Some(&file),
                    public,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn get(&self, name: &str) -> Option<DefinitionKind> {
        let name = name.strip_prefix('.').unwrap_or(name);
        self.map.get(name).map(|e| e.kind)
    }
}
