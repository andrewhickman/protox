//! Interfaces for customizing resolution of protobuf source files.

mod chain;
mod descriptor_set;
mod google;
mod include;
#[cfg(test)]
mod tests;

pub use chain::ChainFileResolver;
pub use descriptor_set::DescriptorSetFileResolver;
pub use google::GoogleFileResolver;
pub use include::IncludeFileResolver;
use prost_types::FileDescriptorProto;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use bytes::{Buf, Bytes};
pub(crate) use include::{check_shadow, path_to_file_name};
use prost::{DecodeError, Message};

use crate::error::Error;

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
    fn open_file(&self, name: &str, file_io: Arc<dyn ProtoxFileIO>) -> Result<File, Error>;
}

/// This trait allows dynamically implement file system for custom IO
pub trait ProtoxFileIO {
    /// This function can be used to implement the custom Input
    fn read_proto(&self, path: &Path) -> anyhow::Result<String>;
}
impl<T> FileResolver for Box<T>
where
    T: FileResolver + ?Sized,
{
    fn resolve_path(&self, path: &Path) -> Option<String> {
        (**self).resolve_path(path)
    }

    fn open_file(&self, name: &str, file_io: Arc<dyn ProtoxFileIO>) -> Result<File, Error> {
        (**self).open_file(name, file_io)
    }
}

/// An opened protobuf source file, returned by [`FileResolver::open_file`].
#[derive(Debug, Clone)]
pub struct File {
    pub(crate) path: Option<PathBuf>,
    pub(crate) source: Option<String>,
    pub(crate) descriptor: FileDescriptorProto,
    pub(crate) encoded: Option<Bytes>,
}

/// Information about a [`File`] after it has been added to a [`Compiler`](crate::Compiler) instance.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub(crate) name: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) is_import: bool,
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
    /// use std::path::Path;
    /// use std::sync::Arc;
    /// # use protox::file::{File, ProtoxFileIO};
    /// # use prost_types::{DescriptorProto, FileDescriptorProto, SourceCodeInfo, source_code_info::Location};
    /// # let tempdir = tempfile::TempDir::new().unwrap();
    /// # std::env::set_current_dir(&tempdir).unwrap();
    /// fs::write("foo.proto", "message Foo { }").unwrap();
    ///
    /// struct MyFileIO;
    ///
    /// impl ProtoxFileIO for MyFileIO {
    ///     fn read_proto(&self, path: &Path) -> anyhow::Result<String> {
    ///         Ok(fs::read_to_string(path)?)
    ///     }
    /// }
    ///
    /// let file_io = Arc::new(MyFileIO{});
    /// let file = File::open("foo.proto", "foo.proto".as_ref(), file_io.clone()).unwrap();
    /// assert_eq!(file.path(), Some("foo.proto".as_ref()));
    /// assert_eq!(file.source(), Some("message Foo { }"));
    /// assert_eq!(file.file_descriptor_proto(), &FileDescriptorProto {
    ///     name: Some("foo.proto".to_owned()),
    ///     message_type: vec![DescriptorProto {
    ///         name: Some("Foo".to_owned()),
    ///         ..Default::default()
    ///     }],
    ///     source_code_info: Some(SourceCodeInfo {
    ///         location: vec![
    ///             Location { path: vec![], span: vec![0, 0, 15], ..Default::default() },
    ///             Location { path: vec![4, 0], span: vec![0, 0, 15], ..Default::default() },
    ///             Location { path: vec![4, 0, 1], span: vec![0, 8, 11], ..Default::default() }
    ///         ]
    ///     }),
    ///     ..Default::default()
    /// });
    ///
    /// assert!(File::open("notfound.proto", "notfound.proto".as_ref(), file_io).unwrap_err().is_file_not_found());
    /// ```
    pub fn open(name: &str, path: &Path, file_io: Arc<dyn ProtoxFileIO>) -> Result<Self, Error> {
        let buf = file_io.read_proto(path).map_err(Error::new)?;
        let descriptor = protox_parse::parse(name, &buf)?;
        Ok(File {
            path: Some(path.to_owned()),
            source: Some(buf),
            descriptor,
            encoded: None,
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
    /// let file = File::from_source("foo.proto", "message Foo { }").unwrap();
    /// assert_eq!(file.path(), None);
    /// assert_eq!(file.source(), Some("message Foo { }"));
    /// assert_eq!(file.file_descriptor_proto(), &FileDescriptorProto {
    ///     name: Some("foo.proto".to_owned()),
    ///     message_type: vec![DescriptorProto {
    ///         name: Some("Foo".to_owned()),
    ///         ..Default::default()
    ///     }],
    ///     source_code_info: Some(SourceCodeInfo {
    ///         location: vec![
    ///             Location { path: vec![], span: vec![0, 0, 15], ..Default::default() },
    ///             Location { path: vec![4, 0], span: vec![0, 0, 15], ..Default::default() },
    ///             Location { path: vec![4, 0, 1], span: vec![0, 8, 11], ..Default::default() }
    ///         ]
    ///     }),
    ///     ..Default::default()
    /// });
    /// ```
    pub fn from_source(name: &str, source: &str) -> Result<Self, Error> {
        let descriptor = protox_parse::parse(name, source)?;

        Ok(File {
            path: None,
            source: Some(source.to_owned()),
            descriptor,
            encoded: None,
        })
    }

    /// Create a new instance of [`File`] from a parsed [`FileDescriptorProto`].
    ///
    /// The file does not need to have type names or imports resolved. Typically, it would be returned by the [`parse()`](protox_parse::parse()) method.
    pub fn from_file_descriptor_proto(file: prost_types::FileDescriptorProto) -> Self {
        File {
            path: None,
            source: None,
            descriptor: file,
            encoded: None,
        }
    }

    /// Create an instance of [`File`] by deserializing a [`FileDescriptorProto`]
    /// from the given bytes.
    ///
    /// Unlike when going through [`from_file_descriptor_proto()`](File::from_file_descriptor_proto), extension options are preserved.
    ///
    /// The file does not need to have type names or imports resolved.
    pub fn decode_file_descriptor_proto<B>(mut buf: B) -> Result<Self, DecodeError>
    where
        B: Buf,
    {
        let encoded = buf.copy_to_bytes(buf.remaining());

        Ok(File {
            path: None,
            source: None,
            descriptor: FileDescriptorProto::decode(encoded.as_ref())?,
            encoded: Some(encoded),
        })
    }

    /// Returns the name of this file.
    pub fn name(&self) -> &str {
        self.descriptor.name()
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
    /// This is typically equivalent to calling [`parse()`](protox_parse::parse()) on the string returned by [`source()`](File::source).
    pub fn file_descriptor_proto(&self) -> &FileDescriptorProto {
        &self.descriptor
    }
}

impl FileMetadata {
    /// Returns the name of this file.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Returns the filesystem path, if this source is backed by a physical file.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns `true` if this file was added explicitly by [`open_file()`](crate::Compiler::open_file), or `false` if it
    /// is was added as an import of some other file.
    pub fn is_import(&self) -> bool {
        self.is_import
    }
}

impl From<FileDescriptorProto> for File {
    fn from(file: FileDescriptorProto) -> Self {
        File::from_file_descriptor_proto(file)
    }
}

impl From<File> for FileDescriptorProto {
    fn from(file: File) -> Self {
        file.descriptor
    }
}
