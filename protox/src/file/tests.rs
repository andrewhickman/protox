use std::{
    io::{self, Seek, Write},
    path::{Path, PathBuf},
};

use prost_types::{source_code_info::Location, FileDescriptorProto, SourceCodeInfo};

use crate::{file::FileResolver, Error};

use super::{
    ChainFileResolver, DescriptorSetFileResolver, File, GoogleFileResolver, IncludeFileResolver,
};

struct EmptyFileResolver;

impl FileResolver for EmptyFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        Err(Error::file_not_found(name))
    }
}

struct SingleFileResolver(File);

impl FileResolver for SingleFileResolver {
    fn resolve_path(&self, path: &Path) -> Option<String> {
        if self.0.path.as_deref() == Some(path) {
            Some(self.0.name().to_owned())
        } else {
            None
        }
    }

    fn open_file(&self, name: &str) -> Result<File, Error> {
        if name == self.0.name() {
            Ok(File::from_file_descriptor_proto(
                self.0.file_descriptor_proto().clone(),
            ))
        } else {
            Err(Error::file_not_found(name))
        }
    }
}

#[test]
fn chain_file_resolver() {
    let source = "syntax = 'proto3';";

    let mut resolver = ChainFileResolver::new();
    resolver.add(EmptyFileResolver);
    resolver.add(SingleFileResolver(
        File::from_source("foo.proto", source).unwrap(),
    ));
    resolver.add(SingleFileResolver(File {
        path: Some(PathBuf::from("./bar.proto")),
        source: Some(source.to_owned()),
        descriptor: protox_parse::parse("bar.proto", source).unwrap(),
        encoded: None,
    }));

    assert_eq!(resolver.resolve_path("./notfound.proto".as_ref()), None);
    assert_eq!(
        resolver.resolve_path("./bar.proto".as_ref()).as_deref(),
        Some("bar.proto")
    );

    assert!(resolver
        .open_file("notfound.proto")
        .unwrap_err()
        .is_file_not_found());
    assert_eq!(resolver.open_file("foo.proto").unwrap().name(), "foo.proto");
}

#[test]
fn descriptor_set_file_resolver() {
    let mut encoded_files: Vec<u8> = vec![
        0x0a, 0x16, 0x0a, 0x09, 0x66, 0x6f, 0x6f, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x62, 0x06,
        0x70, 0x72, 0x6f, 0x74, 0x6f, 0x33,
    ];
    let unknown_field = &[0x90, 0x03, 0x05];
    encoded_files.extend_from_slice(unknown_field);

    let resolver = DescriptorSetFileResolver::decode(encoded_files.as_slice()).unwrap();

    let file = resolver.open_file("foo.proto").unwrap();
    assert_eq!(file.name(), "foo.proto");
    assert_eq!(file.source(), None);
    assert_eq!(file.path(), None);
    assert_eq!(
        file.file_descriptor_proto(),
        &FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        }
    );
    assert!(file
        .encoded
        .unwrap()
        .as_ref()
        .ends_with(unknown_field.as_ref()));

    assert!(resolver
        .open_file("notfound.proto")
        .unwrap_err()
        .is_file_not_found());
}

#[test]
fn google_resolver() {
    let resolver = GoogleFileResolver::new();
    assert_eq!(
        resolver
            .open_file("google/protobuf/any.proto")
            .unwrap()
            .name(),
        "google/protobuf/any.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/api.proto")
            .unwrap()
            .name(),
        "google/protobuf/api.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/descriptor.proto")
            .unwrap()
            .name(),
        "google/protobuf/descriptor.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/duration.proto")
            .unwrap()
            .name(),
        "google/protobuf/duration.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/empty.proto")
            .unwrap()
            .name(),
        "google/protobuf/empty.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/field_mask.proto")
            .unwrap()
            .name(),
        "google/protobuf/field_mask.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/source_context.proto")
            .unwrap()
            .name(),
        "google/protobuf/source_context.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/struct.proto")
            .unwrap()
            .name(),
        "google/protobuf/struct.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/timestamp.proto")
            .unwrap()
            .name(),
        "google/protobuf/timestamp.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/type.proto")
            .unwrap()
            .name(),
        "google/protobuf/type.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/wrappers.proto")
            .unwrap()
            .name(),
        "google/protobuf/wrappers.proto"
    );
    assert_eq!(
        resolver
            .open_file("google/protobuf/compiler/plugin.proto")
            .unwrap()
            .name(),
        "google/protobuf/compiler/plugin.proto"
    );
    assert!(resolver
        .open_file("otherfile")
        .unwrap_err()
        .is_file_not_found());
}

#[test]
fn include_resolver() {
    let include = IncludeFileResolver::new("/path/to/include".into());

    #[cfg(unix)]
    fn non_utf8_path() -> PathBuf {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        OsStr::from_bytes(&[0, 159, 146, 150]).into()
    }

    #[cfg(windows)]
    fn non_utf8_path() -> PathBuf {
        use std::{ffi::OsString, os::windows::ffi::OsStringExt};

        OsString::from_wide(&[0x61, 0xE9, 0x20, 0xD83D, 0xD83D, 0xDCA9]).into()
    }

    assert_eq!(
        include
            .resolve_path(Path::new("/path/to/include/foo.proto"))
            .as_deref(),
        Some("foo.proto")
    );
    assert_eq!(
        include.resolve_path(Path::new("/path/nope/include/foo.proto")),
        None
    );
    assert_eq!(
        include
            .resolve_path(Path::new("/path/./to/include/foo.proto"))
            .as_deref(),
        Some("foo.proto")
    );
    assert_eq!(include.resolve_path(Path::new("/path/to/include")), None);
    assert_eq!(
        include.resolve_path(Path::new("/path/to/../to/include/foo.proto")),
        None
    );
    assert_eq!(include.resolve_path(Path::new("/path/to")), None);
    assert_eq!(
        include
            .resolve_path(Path::new("/path/to/include/dir/foo.proto"))
            .as_deref(),
        Some("dir/foo.proto")
    );
    assert_eq!(
        include
            .resolve_path(Path::new("/path/to/include/./foo.proto"))
            .as_deref(),
        Some("foo.proto")
    );
    assert_eq!(
        include
            .resolve_path(Path::new("/path/to/include/dir/./foo.proto"))
            .as_deref(),
        Some("dir/foo.proto")
    );
    assert_eq!(
        include.resolve_path(Path::new("/path/to/include/dir/../foo.proto")),
        None
    );
    assert_eq!(
        include.resolve_path(&Path::new("/path/to/include").join(non_utf8_path())),
        None
    );

    let include_non_utf8 =
        IncludeFileResolver::new(Path::new("/path/to/include").join(non_utf8_path()));
    assert_eq!(
        include_non_utf8
            .resolve_path(
                &Path::new("/path/to/include")
                    .join(non_utf8_path())
                    .join("foo.proto")
            )
            .as_deref(),
        Some("foo.proto")
    );
}

#[test]
fn file_open() {
    let mut tempfile = tempfile::NamedTempFile::new().unwrap();
    tempfile.write_all("syntax = 'proto3';".as_bytes()).unwrap();
    tempfile.seek(io::SeekFrom::Start(0)).unwrap();

    let file = File::open("foo.proto", tempfile.path()).unwrap();

    assert_eq!(file.name(), "foo.proto");
    assert_eq!(file.path(), Some(tempfile.path()));
    assert_eq!(file.source(), Some("syntax = 'proto3';"));
    assert_eq!(
        file.file_descriptor_proto(),
        &FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            source_code_info: Some(SourceCodeInfo {
                location: vec![
                    Location {
                        path: vec![],
                        span: vec![0, 0, 18],
                        ..Default::default()
                    },
                    Location {
                        path: vec![12],
                        span: vec![0, 0, 18],
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        }
    );
}

#[test]
fn file_from_source() {
    let file = File::from_source("foo.proto", "syntax = 'proto3';").unwrap();

    assert_eq!(file.name(), "foo.proto");
    assert_eq!(file.path(), None);
    assert_eq!(file.source(), Some("syntax = 'proto3';"));
    assert_eq!(
        file.file_descriptor_proto(),
        &FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            source_code_info: Some(SourceCodeInfo {
                location: vec![
                    Location {
                        path: vec![],
                        span: vec![0, 0, 18],
                        ..Default::default()
                    },
                    Location {
                        path: vec![12],
                        span: vec![0, 0, 18],
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        }
    );
}

#[test]
fn file_from_file_descriptor_proto() {
    let file = File::from(FileDescriptorProto {
        name: Some("foo.proto".to_owned()),
        syntax: Some("proto3".to_owned()),
        ..Default::default()
    });

    assert_eq!(file.name(), "foo.proto");
    assert_eq!(file.path(), None);
    assert_eq!(file.source(), None);
    assert_eq!(
        file.file_descriptor_proto(),
        &FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        }
    );

    assert_eq!(
        FileDescriptorProto::from(file),
        FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        }
    );
}

#[test]
fn file_decode_file_descriptor_proto() {
    let file = File::decode_file_descriptor_proto(
        [
            0x0a, 0x09, 0x66, 0x6f, 0x6f, 0x2e, 0x70, 0x72, 0x6f, 0x74, 0x6f, 0x62, 0x06, 0x70,
            0x72, 0x6f, 0x74, 0x6f, 0x33,
        ]
        .as_ref(),
    )
    .unwrap();

    assert_eq!(file.name(), "foo.proto");
    assert_eq!(file.path(), None);
    assert_eq!(file.source(), None);
    assert_eq!(
        file.file_descriptor_proto(),
        &FileDescriptorProto {
            name: Some("foo.proto".to_owned()),
            syntax: Some("proto3".to_owned()),
            ..Default::default()
        }
    );
}

#[test]
fn file_decode_file_descriptor_proto_err() {
    let invalid = b"invalid";
    assert!(File::decode_file_descriptor_proto(invalid.as_ref()).is_err());
}
