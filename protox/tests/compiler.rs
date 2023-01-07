use std::fs;

use insta::assert_yaml_snapshot;
use miette::{Diagnostic, JSONReportHandler};
use prost::Message;
use prost_reflect::{DescriptorPool, Value};
use prost_types::{
    source_code_info::Location, FileDescriptorProto, FileDescriptorSet, SourceCodeInfo,
};
use protox::{
    file::{ChainFileResolver, DescriptorSetFileResolver, File, FileResolver, GoogleFileResolver},
    Compiler, Error,
};

struct TestFileResolver {
    files: &'static [(&'static str, &'static str)],
}

impl FileResolver for TestFileResolver {
    fn open_file(&self, name: &str) -> Result<File, Error> {
        if name == "customerror.proto" {
            return Err(Error::new("failed to load file!"));
        }

        for file in self.files {
            if file.0 == name {
                return File::from_source(name, file.1);
            }
        }

        Err(Error::file_not_found(name))
    }
}

fn check(files: &'static [(&'static str, &'static str)]) -> Result<Compiler, Error> {
    let tempdir = tempfile::tempdir().unwrap();
    for (file, source) in files {
        fs::write(tempdir.path().join(file), source).unwrap();
    }

    let mut compiler = Compiler::with_file_resolver(TestFileResolver { files });
    for (file, _) in &files[..files.len() - 1] {
        compiler.open_file(file).unwrap();
    }

    compiler.open_file(files[files.len() - 1].0)?;
    Ok(compiler)
}

fn check_err(files: &'static [(&'static str, &'static str)]) -> serde_json::Value {
    error_to_json(&check(files).unwrap_err())
}

fn error_to_json(err: &dyn Diagnostic) -> serde_json::Value {
    let mut json = String::new();
    JSONReportHandler::new()
        .render_report(&mut json, err)
        .unwrap();
    serde_json::from_str(&json).unwrap()
}

#[test]
fn import_not_found() {
    assert_yaml_snapshot!(check_err(&[("root.proto", "import 'notfound.proto';")]));
}

#[test]
fn import_error() {
    assert_yaml_snapshot!(check_err(&[("root.proto", "import 'customerror.proto';")]));
}

#[test]
fn type_not_found() {
    assert_yaml_snapshot!(check_err(&[(
        "root.proto",
        "
        message Foo {
            optional NotFound foo = 1;
        }
    "
    )]));
}

#[test]
fn default_options() {
    let mut compiler = Compiler::with_file_resolver(TestFileResolver {
        files: &[("dep.proto", ""), ("root.proto", "import 'dep.proto';")],
    });

    compiler.open_file("root.proto").unwrap();

    let files = compiler.file_descriptor_set();
    assert_eq!(
        files,
        FileDescriptorSet {
            file: vec![FileDescriptorProto {
                name: Some("root.proto".to_owned()),
                dependency: vec!["dep.proto".to_owned()],
                ..Default::default()
            },],
        }
    );

    let encoded = compiler.encode_file_descriptor_set();
    assert_eq!(
        FileDescriptorSet::decode(encoded.as_slice()).unwrap(),
        files
    );
}

#[test]
fn include_imports() {
    let mut compiler = Compiler::with_file_resolver(TestFileResolver {
        files: &[("dep.proto", ""), ("root.proto", "import 'dep.proto';")],
    });

    compiler.include_imports(true);
    compiler.open_file("root.proto").unwrap();

    let files = compiler.file_descriptor_set();
    assert_eq!(
        files,
        FileDescriptorSet {
            file: vec![
                FileDescriptorProto {
                    name: Some("dep.proto".to_owned()),
                    ..Default::default()
                },
                FileDescriptorProto {
                    name: Some("root.proto".to_owned()),
                    dependency: vec!["dep.proto".to_owned()],
                    ..Default::default()
                },
            ],
        }
    );

    let encoded = compiler.encode_file_descriptor_set();
    assert_eq!(
        FileDescriptorSet::decode(encoded.as_slice()).unwrap(),
        files
    );
}

#[test]
fn include_source_info() {
    let mut compiler = Compiler::with_file_resolver(TestFileResolver {
        files: &[("dep.proto", ""), ("root.proto", "import 'dep.proto';")],
    });

    compiler.include_source_info(true);
    compiler.open_file("root.proto").unwrap();

    let files = compiler.file_descriptor_set();
    assert_eq!(
        files,
        FileDescriptorSet {
            file: vec![FileDescriptorProto {
                name: Some("root.proto".to_owned()),
                dependency: vec!["dep.proto".to_owned()],
                source_code_info: Some(SourceCodeInfo {
                    location: vec![
                        Location {
                            path: vec![],
                            span: vec![0, 0, 19],
                            ..Default::default()
                        },
                        Location {
                            path: vec![3, 0],
                            span: vec![0, 0, 19],
                            ..Default::default()
                        }
                    ]
                }),
                ..Default::default()
            },],
        }
    );

    let encoded = compiler.encode_file_descriptor_set();
    assert_eq!(
        FileDescriptorSet::decode(encoded.as_slice()).unwrap(),
        files
    );
}

#[test]
fn pass_through_extension_options() {
    let mut resolver = ChainFileResolver::new();
    resolver.add(TestFileResolver {
        files: &[(
            "root.proto",
            "
            import 'google/protobuf/descriptor.proto';

            extend google.protobuf.FileOptions {
                optional int32 ext = 1001;
            }

            option (ext) = 1;
        ",
        )],
    });
    resolver.add(GoogleFileResolver::new());

    let mut compiler = Compiler::with_file_resolver(resolver);
    compiler.include_imports(true);
    compiler.open_file("root.proto").unwrap();

    let dyn_set = DescriptorPool::decode(compiler.encode_file_descriptor_set().as_slice()).unwrap();
    let ext = dyn_set.get_extension_by_name("ext").unwrap();
    assert_eq!(
        dyn_set
            .get_file_by_name("root.proto")
            .unwrap()
            .options()
            .get_extension(&ext)
            .as_ref(),
        &Value::I32(1)
    );

    let roundtripped_resolver =
        DescriptorSetFileResolver::decode(compiler.encode_file_descriptor_set().as_slice())
            .unwrap();
    let mut roundtripped_compiler = Compiler::with_file_resolver(roundtripped_resolver);
    roundtripped_compiler.include_imports(true);

    roundtripped_compiler.open_file("root.proto").unwrap();
    let roundtripped_dyn_set = DescriptorPool::decode(
        roundtripped_compiler
            .encode_file_descriptor_set()
            .as_slice(),
    )
    .unwrap();
    let roundtripped_ext = roundtripped_dyn_set.get_extension_by_name("ext").unwrap();
    assert_eq!(
        roundtripped_dyn_set
            .get_file_by_name("root.proto")
            .unwrap()
            .options()
            .get_extension(&roundtripped_ext)
            .as_ref(),
        &Value::I32(1)
    );
}
