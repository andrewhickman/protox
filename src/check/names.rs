use std::collections::{hash_map, HashMap};

use logos::Span;

use super::{CheckError, Definition};

/// A simple map of all definitions in a proto file for checking downstream files.
#[derive(Debug)]
pub(crate) struct NameMap {
    map: HashMap<String, Entry>,
}

#[derive(Debug, Clone)]
struct Entry {
    def: Definition,
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
        def: Definition,
        file: Option<&str>,
        public: bool,
    ) -> Result<(), CheckError> {
        assert!(!matches!(def, Definition::Extend { .. }));
        match self.map.entry(name) {
            hash_map::Entry::Vacant(entry) => {
                entry.insert(Entry {
                    file: file.map(ToOwned::to_owned),
                    def,
                    public: true,
                });
                Ok(())
            }
            hash_map::Entry::Occupied(entry) => match (&def, &entry.get().def) {
                (Definition::Package { .. }, Definition::Package { .. }) => Ok(()),
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
                                second: def.name_span(),
                            }
                        }
                    } else {
                        CheckError::DuplicateNameInFile {
                            name,
                            first: entry.get().def.name_span(),
                            second: def.name_span(),
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
                self.add(name.clone(), entry.def.clone(), Some(&file), public)?;
            }
        }
        Ok(())
    }

    pub(super) fn get(&self, name: &str) -> Option<&Definition> {
        self.map.get(name).map(|e| &e.def)
    }
}

impl Definition {
    fn name_span(&self) -> Span {
        match self {
            Definition::Package { name_span, .. }
            | Definition::Message { name_span, .. }
            | Definition::Enum { name_span, .. }
            | Definition::Service { name_span, .. }
            | Definition::Oneof { name_span, .. }
            | Definition::Group { name_span, .. } => name_span.clone(),
            Definition::Extend { .. } => unimplemented!("extend has no name"),
        }
    }
}
