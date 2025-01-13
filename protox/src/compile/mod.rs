use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Write},
    path::{Path, PathBuf},
};

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage, Value};
use prost_types::{FileDescriptorProto, FileDescriptorSet};

use crate::{
    error::{Error, ErrorKind},
    file::{check_shadow, path_to_file_name, File, FileMetadata, FileResolver},
};

#[cfg(test)]
mod tests;

/// Options for compiling protobuf files.
///
/// # Examples
///
/// ```
/// # use std::fs;
/// # use prost_types::{
/// #    DescriptorProto, FieldDescriptorProto, field_descriptor_proto::{Label, Type}, FileDescriptorSet, FileDescriptorProto,
/// #    SourceCodeInfo, source_code_info::Location
/// # };
/// # use protox::Compiler;
/// # fn main() -> Result<(), protox::Error> {
/// # let tempdir = tempfile::TempDir::new().unwrap();
/// # std::env::set_current_dir(&tempdir).unwrap();
/// #
/// fs::write("bar.proto", "
///     message Bar { }
/// ").unwrap();
///
/// let file_descriptor_set = Compiler::new(["."])?
///     .include_imports(true)
///     .include_source_info(false)
///     .open_file("bar.proto")?
///     .file_descriptor_set();
///
/// assert_eq!(file_descriptor_set.file[0].message_type[0].name(), "Bar");
/// # Ok(())
/// # }
/// ```
pub struct Compiler {
    pool: DescriptorPool,
    resolver: Box<dyn FileResolver>,
    files: HashMap<String, FileMetadata>,
    include_imports: bool,
    include_source_info: bool,
}

impl Compiler {
    /// Creates a new [`Compiler`] with default options and the given set of include paths.
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

    /// Creates a new [`Compiler`] with a custom [`FileResolver`] for looking up imported files.
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

    /// Sets whether the output `FileDescriptorSet` should include source info.
    ///
    /// If set, the file descriptors returned by [`file_descriptor_set`](Compiler::file_descriptor_set) will have
    /// the [`FileDescriptorProto::source_code_info`](prost_types::FileDescriptorProto::source_code_info) field
    /// populated with source locations and comments.
    pub fn include_source_info(&mut self, yes: bool) -> &mut Self {
        self.include_source_info = yes;
        self
    }

    /// Sets whether the output `FileDescriptorSet` should include imported files.
    ///
    /// By default, only files explicitly added with [`open_file`](Compiler::open_file) are returned by [`file_descriptor_set`](Compiler::file_descriptor_set).
    /// If this option is set, imported files are included too.
    pub fn include_imports(&mut self, yes: bool) -> &mut Self {
        self.include_imports = yes;
        self
    }

    /// Compiles the file at the given path, and adds it to this `Compiler` instance.
    ///
    /// If the path is absolute, or relative to the current directory, it must reside under one of the
    /// include paths. Otherwise, it is looked up relative to the given include paths in the same way as
    /// `import` statements.
    pub fn open_file(&mut self, path: impl AsRef<Path>) -> Result<&mut Self, Error> {
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

        if let Some(file_metadata) = self.files.get_mut(&name) {
            if is_resolved {
                check_shadow(&name, file_metadata.path(), path)?;
            }
            file_metadata.is_import = false;
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
            check_shadow(&name, file.path(), path)?;
        }

        let mut import_stack = vec![name.clone()];
        let mut already_imported = HashSet::new();
        for (i, import) in file.descriptor.dependency.iter().enumerate() {
            if !already_imported.insert(import) {
                return Err(Error::duplicated_import(import.to_owned(), &file, i));
            }
            self.add_import(import, &mut import_stack)
                .map_err(|e| e.into_import_error(&file, i))?;
        }

        let path = self.check_file(file)?;
        self.files.insert(
            name.clone(),
            FileMetadata {
                name,
                path,
                is_import: false,
            },
        );
        Ok(self)
    }

    /// Compiles the given files, and adds them to this `Compiler` instance.
    ///
    /// See [`open_file()`][Compiler::open_file()].
    pub fn open_files(
        &mut self,
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<&mut Self, Error> {
        for path in paths {
            self.open_file(path)?;
        }

        Ok(self)
    }

    /// Converts all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet).
    ///
    /// Files are sorted topologically, with dependency files ordered before the files that import them.
    pub fn file_descriptor_set(&self) -> prost_types::FileDescriptorSet {
        let file = self
            .pool
            .files()
            .filter(|f| self.include_imports || !self.files[f.name()].is_import)
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

    /// Converts all added files into an instance of [`FileDescriptorSet`](prost_types::FileDescriptorSet) and encodes it.
    ///
    /// This is equivalent to `file_descriptor_set()?.encode_to_vec()`, with the exception that extension
    /// options are included.
    pub fn encode_file_descriptor_set(&self) -> Vec<u8> {
        if self.include_imports && self.include_source_info {
            // Avoid reflection if possible.
            return self.pool.encode_to_vec();
        }

        let file_desc = FileDescriptorProto::default().descriptor();

        let files = self
            .pool
            .files()
            .filter(|f| self.include_imports || !self.files[f.name()].is_import)
            .map(|f| {
                let file_buf = f.encode_to_vec();

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

    /// Gets a copy of the [`DescriptorPool`] containing all referenced files.
    pub fn descriptor_pool(&self) -> DescriptorPool {
        self.pool.clone()
    }

    /// Gets a reference to all imported source files.
    ///
    /// The files will appear in topological order, so each file appears before any file that imports it.
    pub fn files(&self) -> impl ExactSizeIterator<Item = &'_ FileMetadata> {
        self.pool.files().map(|f| &self.files[f.name()])
    }

    fn add_import(&mut self, file_name: &str, import_stack: &mut Vec<String>) -> Result<(), Error> {
        if import_stack.iter().any(|name| name == file_name) {
            let mut cycle = String::new();
            for import in import_stack {
                write!(&mut cycle, "{} -> ", import).unwrap();
            }
            write!(&mut cycle, "{}", file_name).unwrap();

            return Err(Error::from_kind(ErrorKind::CircularImport {
                name: file_name.to_owned(),
                cycle,
            }));
        }

        if self.files.contains_key(file_name) {
            return Ok(());
        }

        let file = self.resolver.open_file(file_name)?;

        import_stack.push(file_name.to_owned());
        let mut already_imported = HashSet::new();
        for (i, import) in file.descriptor.dependency.iter().enumerate() {
            if !already_imported.insert(import) {
                return Err(Error::duplicated_import(import.to_owned(), &file, i));
            }
            self.add_import(import, import_stack)
                .map_err(|e| e.into_import_error(&file, i))?;
        }
        import_stack.pop();

        let path = self.check_file(file)?;
        self.files.insert(
            file_name.to_owned(),
            FileMetadata {
                name: file_name.to_owned(),
                path,
                is_import: true,
            },
        );
        Ok(())
    }

    fn check_file(
        &mut self,
        File {
            path,
            source,
            descriptor,
            encoded,
        }: File,
    ) -> Result<Option<PathBuf>, Error> {
        if let Some(encoded) = &encoded {
            self.pool.decode_file_descriptor_proto(encoded.clone())
        } else {
            self.pool.add_file_descriptor_proto(descriptor)
        }
        .map_err(|mut err| {
            if let Some(source) = source {
                err = err.with_source_code(&source);
            }
            err
        })?;

        Ok(path)
    }
}

impl fmt::Debug for Compiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compiler")
            .field("include_imports", &self.include_imports)
            .field("include_source_info", &self.include_source_info)
            .finish_non_exhaustive()
    }
}
