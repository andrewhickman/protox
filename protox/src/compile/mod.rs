use std::{
    collections::HashMap,
    fmt::{self, Write},
    path::{Path, PathBuf},
};

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage, Value};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::{
    error::{Error, ErrorKind},
    file::{check_shadow, path_to_file_name, File, FileResolver},
};

#[cfg(test)]
mod tests;

/// Options for compiling protobuf files.
pub struct Compiler {
    pool: DescriptorPool,
    resolver: Box<dyn FileResolver>,
    files: HashMap<String, ParsedFile>,
    include_imports: bool,
    include_source_info: bool,
}

#[derive(Debug)]
struct ParsedFile {
    path: Option<PathBuf>,
    is_root: bool,
}

impl Compiler {
    /// Create a new [`Compiler`] with default options and the given set of include paths.
    ///
    /// In addition to the given include paths, the [`Compiler`] instance will be able to import
    /// standard files like `google/protobuf/descriptor.proto`.
    pub fn new<I, P>(includes: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        use crate::file::{ChainFileResolver, GoogleFileResolver, IncludeFileResolver};

        let mut resolver = ChainFileResolver::new();

        for include in includes {
            resolver.add(IncludeFileResolver::new(include.as_ref().to_owned()));
        }

        resolver.add(GoogleFileResolver::new());

        Ok(Compiler::with_file_resolver(resolver))
    }

    /// Create a new [`Compiler`] with a custom [`FileResolver`] for looking up imported files.
    pub fn with_file_resolver<R>(resolver: R) -> Self
    where
        R: FileResolver + 'static,
    {
        Compiler {
            pool: DescriptorPool::new(),
            resolver: Box::new(resolver),
            files: HashMap::new(),
            include_imports: false,
            include_source_info: false,
        }
    }

    /// Set whether the output `FileDescriptorSet` should include source info.
    ///
    /// If set, the file descriptors returned by [`file_descriptor_set`](Compiler::file_descriptor_set) will have
    /// the [`FileDescriptorProto::source_code_info`](prost_types::FileDescriptorProto::source_code_info) field
    /// populated with source locations and comments.
    pub fn include_source_info(&mut self, yes: bool) -> &mut Self {
        self.include_source_info = yes;
        self
    }

    /// Set whether the output `FileDescriptorSet` should include imported files.
    ///
    /// By default, only files explicitly added with [`add_file`](Compiler::add_file) are returned by [`file_descriptor_set`](Compiler::file_descriptor_set).
    /// If this option is set, imported files are included too.
    pub fn include_imports(&mut self, yes: bool) -> &mut Self {
        self.include_imports = yes;
        self
    }

    /// Compile the file at the given path, and add it to this `Compiler` instance.
    ///
    /// If the path is absolute, or relative to the current directory, it must reside under one of the
    /// include paths. Otherwise, it is looked up relative to the given include paths in the same way as
    /// `import` statements.
    pub fn add_file(&mut self, path: impl AsRef<Path>) -> Result<&mut Self, Error> {
        let path = path.as_ref();
        let (name, is_resolved) = if let Some(name) = self.resolver.resolve_path(path) {
            (name, true)
        } else if let Some(name) = path_to_file_name(path) {
            (name, false)
        } else {
            return Err(Error::from_kind(ErrorKind::FileNotIncluded {
                path: path.to_owned(),
            }));
        };

        if let Some(parsed_file) = self.files.get_mut(&name) {
            if is_resolved {
                check_shadow(parsed_file.path.as_deref(), path)?;
            }
            parsed_file.is_root = true;
            return Ok(self);
        }

        let file = self.resolver.open_file(&name).map_err(|err| {
            if err.is_file_not_found() {
                Error::from_kind(ErrorKind::FileNotIncluded {
                    path: path.to_owned(),
                })
            } else {
                err
            }
        })?;
        if is_resolved {
            check_shadow(file.path(), path)?;
        }

        let mut import_stack = vec![name.clone()];
        for import in &file.descriptor.dependency {
            self.add_import(import, &mut import_stack)?;
        }
        drop(import_stack);

        let path = file.path.clone();

        self.check_file(file)?;
        self.files.insert(
            name,
            ParsedFile {
                path,
                is_root: true,
            },
        );
        Ok(self)
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet).
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn file_descriptor_set(&self) -> prost_types::FileDescriptorSet {
        let file = self
            .pool
            .files()
            .filter(|f| self.include_imports || self.files[f.name()].is_root)
            .map(|f| {
                if self.include_source_info {
                    f.file_descriptor_proto().clone()
                } else {
                    prost_types::FileDescriptorProto {
                        source_code_info: None,
                        ..f.file_descriptor_proto().clone()
                    }
                }
            })
            .collect();

        prost_types::FileDescriptorSet { file }
    }

    /// Convert all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet) and encodes it.
    ///
    /// This is equivalent to `file_descriptor_set()?.encode_to_vec()`, with the exception that extension
    /// options are included.
    pub fn encode_file_descriptor_set(&self) -> Vec<u8> {
        let file_desc = FileDescriptorProto::default().descriptor();

        let files = self
            .pool
            .files()
            .filter(|f| self.include_imports || self.files[f.name()].is_root)
            .map(|f| {

                let mut file_buf = Vec::new();
                f.encode(&mut file_buf).unwrap();

                let mut file_msg =
                    DynamicMessage::decode(file_desc.clone(), file_buf.as_slice()).unwrap();
                if !self.include_source_info {
                    file_msg.clear_field_by_name("source_code_info");
                }

                Value::Message(file_msg)
            })
            .collect();

        let mut file_descriptor_set = FileDescriptorSet::default().transcode_to_dynamic();
        file_descriptor_set.set_field_by_name("file", Value::List(files));
        file_descriptor_set.encode_to_vec()
    }

    fn add_import(&mut self, file_name: &str, import_stack: &mut Vec<String>) -> Result<(), Error> {
        if import_stack.iter().any(|name| name == file_name) {
            let mut cycle = String::new();
            for import in import_stack {
                write!(&mut cycle, "{} -> ", import).unwrap();
            }
            write!(&mut cycle, "{}", file_name).unwrap();

            return Err(Error::from_kind(ErrorKind::CircularImport { cycle }));
        }

        if self.files.contains_key(file_name) {
            return Ok(());
        }

        let file = self.resolver.open_file(file_name)?;

        import_stack.push(file_name.to_owned());
        for import in &file.descriptor.dependency {
            self.add_import(import, import_stack)?;
        }
        import_stack.pop();

        let path = file.path.clone();

        self.check_file(file)?;
        self.files.insert(
            file_name.to_owned(),
            ParsedFile {
                path,
                is_root: false,
            },
        );
        Ok(())
    }

    fn check_file(&mut self, file: File) -> Result<(), Error> {
        if let Some(encoded) = file.encoded {
            self.pool.decode_file_descriptor_proto(encoded)
        } else {
            self.pool.add_file_descriptor_proto(file.descriptor)
        }
        .map_err(|mut err| {
            if let Some(source) = file.source.as_deref() {
                err = err.with_source_code(source);
            }
            Error::from_kind(ErrorKind::Check { err })
        })?;

        Ok(())
    }
}

impl fmt::Debug for Compiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compiler")
            .field("files", &self.files)
            .field("include_imports", &self.include_imports)
            .field("include_source_info", &self.include_source_info)
            .finish_non_exhaustive()
    }
}
