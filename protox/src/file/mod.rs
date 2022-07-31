//! Handling of protobuf source files

mod chain;
mod descriptor_set;
mod google;
mod include;

pub use chain::ChainFileResolver;
pub use descriptor_set::DescriptorSetFileResolver;
// TODO compiler descriptor sets at build time
pub use google::GoogleFileResolver;
pub use include::IncludeFileResolver;

use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

use bytes::Buf;
pub(crate) use include::check_shadow;
use prost::{DecodeError, Message};

use crate::{
    error::{DynSourceCode, ErrorKind},
    transcode_file,
    types::FileDescriptorProto,
    Error, MAX_FILE_LEN,
};

/// A strategy for locating protobuf source files.
///
/// The main implementation is [`IncludeFileResolver`] which uses the file system, but
/// this trait allows sourcing files from other places as well.
pub trait FileResolver {
    /// Converts a file system path to a unique file name.
    fn resolve_path(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Opens a file by its unique name.
    ///
    /// # Errors
    ///
    /// If the file is not found, the implementation should return [`Error::file_not_found`].
    fn open_file(&self, name: &str) -> Result<File, Error>;
}

impl<T> FileResolver for Box<T>
where
    T: FileResolver + ?Sized,
{
    fn resolve_path(&self, path: &Path) -> Option<String> {
        (**self).resolve_path(path)
    }

    fn open_file(&self, name: &str) -> Result<File, Error> {
        (**self).open_file(name)
    }
}

/// An opened protobuf source file, returned by [`FileResolver::open_file`].
#[derive(Debug, Clone)]
pub struct File {
    pub(crate) path: Option<PathBuf>,
    pub(crate) source: Option<String>,
    pub(crate) descriptor: FileDescriptorProto,
}

impl File {
    /// Read a protobuf source file from the filesystem into a new instance of [`File`]
    ///
    /// # Errors
    ///
    /// Returns an error if there is an IO error opening the file, or it is not
    /// a valid protobuf source file.
    ///
    /// If the file does not exist, [`Error::file_not_found()`] is returned
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::{fs, path::PathBuf};
    /// # use protox::file::File;
    /// # use prost_types::{DescriptorProto, FileDescriptorProto, SourceCodeInfo, source_code_info::Location};
    /// # let tempdir = assert_fs::TempDir::new().unwrap();
    /// # std::env::set_current_dir(&tempdir).unwrap();
    /// fs::write("foo.proto", "message Foo { }").unwrap();
    ///
    /// let file = File::open("foo.proto".as_ref()).unwrap();
    /// assert_eq!(file.path(), Some("foo.proto".as_ref()));
    /// assert_eq!(file.source(), Some("message Foo { }"));
    /// assert_eq!(file.to_file_descriptor_proto(), FileDescriptorProto {
    ///     message_type: vec![DescriptorProto {
    ///         name: Some("Foo".to_owned()),
    ///         ..Default::default()
    ///     }],
    ///     source_code_info: Some(SourceCodeInfo {
    ///         location: vec![
    ///             /* ... */
    /// #           Location { path: vec![], span: vec![0, 0, 15], ..Default::default() },
    /// #           Location { path: vec![4, 0], span: vec![0, 0, 15], ..Default::default() },
    /// #           Location { path: vec![4, 0, 1], span: vec![0, 8, 11], ..Default::default() }
    ///         ]
    ///     }),
    ///     ..Default::default()
    /// });
    ///
    /// assert!(File::open("notfound.proto".as_ref()).unwrap_err().is_file_not_found());
    /// ```
    pub fn open(path: &Path) -> Result<Self, Error> {
        let map_io_err = |err: io::Error| -> Error {
            Error::from_kind(ErrorKind::OpenFile {
                path: path.to_owned(),
                err,
                src: DynSourceCode::default(),
                span: None,
            })
        };

        let file = fs::File::open(&path).map_err(map_io_err)?;
        let metadata = file.metadata().map_err(map_io_err)?;

        if metadata.len() > MAX_FILE_LEN {
            return Err(Error::from_kind(ErrorKind::FileTooLarge {
                src: DynSourceCode::default(),
                span: None,
            }));
        }

        let mut buf = String::with_capacity(metadata.len() as usize);
        file.take(MAX_FILE_LEN)
            .read_to_string(&mut buf)
            .map_err(map_io_err)?;

        let descriptor = protox_parse::parse(&buf)?;

        Ok(File {
            path: Some(path.to_owned()),
            source: Some(buf),
            descriptor: transcode_file(&descriptor, &mut Vec::new()),
        })
    }

    /// Read a protobuf source file from a string into a new instance of [`File`]
    ///
    /// # Errors
    ///
    /// Returns an error the string is not a valid protobuf source file.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::{fs, path::PathBuf};
    /// # use protox::file::File;
    /// # use prost_types::{DescriptorProto, FileDescriptorProto, SourceCodeInfo, source_code_info::Location};
    /// let file = File::from_source("message Foo { }").unwrap();
    /// assert_eq!(file.path(), None);
    /// assert_eq!(file.source(), Some("message Foo { }"));
    /// assert_eq!(file.to_file_descriptor_proto(), FileDescriptorProto {
    ///     message_type: vec![DescriptorProto {
    ///         name: Some("Foo".to_owned()),
    ///         ..Default::default()
    ///     }],
    ///     source_code_info: Some(SourceCodeInfo {
    ///         location: vec![
    ///             /* ... */
    /// #           Location { path: vec![], span: vec![0, 0, 15], ..Default::default() },
    /// #           Location { path: vec![4, 0], span: vec![0, 0, 15], ..Default::default() },
    /// #           Location { path: vec![4, 0, 1], span: vec![0, 8, 11], ..Default::default() }
    ///         ]
    ///     }),
    ///     ..Default::default()
    /// });
    /// ```
    pub fn from_source(source: &str) -> Result<Self, Error> {
        let descriptor = protox_parse::parse(source)?;

        Ok(File {
            path: None,
            source: Some(source.to_owned()),
            descriptor: transcode_file(&descriptor, &mut Vec::new()),
        })
    }

    /// Create a new instance of [`File`] from a parsed [`FileDescriptorProto`](prost_types::FileDescriptorProto).
    ///
    /// The file does not need to have type names or imports resolved. Typically, it would be returned by the [`parse()`](crate::parse()) method.
    pub fn from_file_descriptor_proto(file: prost_types::FileDescriptorProto) -> Self {
        File {
            path: None,
            source: None,
            descriptor: transcode_file(&file, &mut Vec::new()),
        }
    }

    /// Create an instance of [`File`] by deserializing a [`FileDescriptorSet`](prost_types::FileDescriptorSet)
    /// from the given bytes.
    ///
    /// Unlike when going through [`from_file_descriptor_proto()`](File::from_file_descriptor_proto), extension options are preserved.
    ///
    /// The file does not need to have type names or imports resolved.
    pub fn decode_file_descriptor_proto<B>(buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        Ok(File {
            path: None,
            source: None,
            descriptor: FileDescriptorProto::decode(buf)?,
        })
    }

    /// Returns the filesystem path, if this source is backed by a physical file.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the full content of the source file if available.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns the parsed value of the source file.
    ///
    /// This is typically equivalent to calling [`parse()`](crate::parse()) on the string returned by [`source()`](File::source).
    pub fn to_file_descriptor_proto(&self) -> prost_types::FileDescriptorProto {
        transcode_file(&self.descriptor, &mut Vec::new())
    }
}

pub(crate) fn path_to_file_name(path: &Path) -> Option<String> {
    let mut name = String::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(component) => {
                if let Some(component) = component.to_str() {
                    if !name.is_empty() {
                        name.push('/');
                    }
                    name.push_str(component);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    Some(name)
}
