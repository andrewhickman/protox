use std::fs;

use insta::assert_yaml_snapshot;
use miette::{Diagnostic, JSONReportHandler};
use protox::{
    file::{File, FileResolver},
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

fn check_err(files: &'static [(&'static str, &'static str)]) -> serde_json::Value {
    let tempdir = tempfile::tempdir().unwrap();
    for (file, source) in files {
        fs::write(tempdir.path().join(file), source).unwrap();
    }

    let mut compiler = Compiler::with_file_resolver(TestFileResolver { files });
    for (file, _) in &files[..files.len() - 1] {
        compiler.open_file(file).unwrap();
    }

    let err = compiler.open_file(files[files.len() - 1].0).unwrap_err();
    error_to_json(&err)
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
