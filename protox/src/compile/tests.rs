use std::{fs, iter::once};

use tempfile::TempDir;

use super::*;

const EMPTY: &[u8] = &[];
const INVALID_UTF8: &[u8] = &[255];

fn with_current_dir(path: impl AsRef<Path>, f: impl FnOnce()) {
    use std::{
        env::{current_dir, set_current_dir},
        sync::Mutex,
    };

    use once_cell::sync::Lazy;
    use scopeguard::defer;

    static CURRENT_DIR_LOCK: Lazy<Mutex<()>> = Lazy::new(Default::default);

    let _lock = CURRENT_DIR_LOCK
        .lock()
        .unwrap_or_else(|err| err.into_inner());

    let prev_dir = current_dir().unwrap();
    defer!({
        let _ = set_current_dir(prev_dir);
    });

    set_current_dir(path).unwrap();
    f();
}

fn test_compile_success(include: impl AsRef<Path>, file: impl AsRef<Path>, name: &str) {
    let include = include.as_ref();
    let file = file.as_ref();

    std::fs::create_dir_all(include).unwrap();
    if let Some(parent) = include.join(name).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(include.join(name), EMPTY).unwrap();

    let mut compiler = Compiler::new(once(include)).unwrap();
    compiler.open_file(file).unwrap();

    assert_eq!(compiler.files().len(), 1);
    assert_eq!(compiler.descriptor_pool().files().len(), 1);
    assert_eq!(
        compiler.file_descriptor_set().file[0],
        prost_types::FileDescriptorProto {
            name: Some(name.to_owned()),
            ..Default::default()
        }
    );
    assert_eq!(
        compiler.files[name].path(),
        Some(include.join(name).as_ref())
    );
}

fn test_compile_error(
    include: impl AsRef<Path>,
    file: impl AsRef<Path>,
    name: &str,
    expected_err: ErrorKind,
) {
    let include = include.as_ref();
    let file = file.as_ref();

    std::fs::create_dir_all(include).unwrap();
    if let Some(parent) = include.join(name).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(include.join(name), EMPTY).unwrap();

    let mut compiler = Compiler::new(once(include)).unwrap();
    let err = compiler.open_file(file).unwrap_err();

    match (err.kind(), &expected_err) {
        (
            ErrorKind::FileNotIncluded { path: lpath },
            ErrorKind::FileNotIncluded { path: rpath },
        ) => assert_eq!(lpath, rpath),
        (err, _) => panic!("unexpected error: {}", err),
    }
    assert_eq!(compiler.files().len(), 0);
}

#[test]
fn abs_include_simple_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.path(), "foo.proto", "foo.proto");
}

#[test]
fn abs_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.path(), "dir/foo.proto", "dir/foo.proto");
}

#[test]
fn abs_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(dir.path(), "foo.proto", "foo.proto");
    })
}

#[test]
fn abs_include_rel_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            dir.path(),
            Path::new("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    })
}

#[test]
fn abs_include_abs_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.path(), dir.path().join("foo.proto"), "foo.proto");
}

#[test]
fn abs_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.path(),
        dir.path().join("dir").join("foo.proto"),
        "dir/foo.proto",
    );
}

#[test]
fn abs_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path(),
            Path::new(".").join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new(".").join("foo.proto"),
            },
        )
    })
}

#[test]
fn abs_include_dot_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path(),
            Path::new(".").join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new(".").join("dir").join("foo.proto"),
            },
        )
    })
}

#[test]
fn abs_subdir_include_simple_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.path().join("include"), "foo.proto", "foo.proto");
}

#[test]
fn abs_subdir_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.path().join("include"), "dir/foo.proto", "dir/foo.proto");
}

#[test]
fn abs_subdir_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path().join("include"),
            Path::new("include").join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new("include").join("foo.proto"),
            },
        );
    });
}

#[test]
fn abs_subdir_include_rel_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path().join("include"),
            Path::new("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new("include").join("dir").join("foo.proto"),
            },
        );
    });
}

#[test]
fn abs_subdir_include_abs_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(&dir, dir.path().join("foo.proto"), "foo.proto");
}

#[test]
fn abs_subdir_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.path().join("include"),
        dir.path().join("include").join("foo.proto"),
        "foo.proto",
    );
}

#[test]
fn abs_subdir_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path().join("include"),
            Path::new(".").join("include").join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new(".").join("include").join("foo.proto"),
            },
        );
    });
}

#[test]
fn abs_subdir_include_dot_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.path().join("include"),
            Path::new(".").join("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new(".").join("include").join("dir").join("foo.proto"),
            },
        );
    });
}

#[test]
fn abs_include_complex_file() {
    let dir = TempDir::new().unwrap();
    test_compile_error(
        &dir,
        dir.path()
            .join("dir")
            .join("..")
            .join("dir")
            .join("foo.proto"),
        "dir/foo.proto",
        ErrorKind::FileNotIncluded {
            path: dir
                .path()
                .join("dir")
                .join("..")
                .join("dir")
                .join("foo.proto"),
        },
    );
}

#[test]
fn abs_subdir_include_complex_file() {
    let dir = TempDir::new().unwrap();
    test_compile_error(
        dir.path().join("include"),
        dir.path()
            .join("include")
            .join("..")
            .join("include")
            .join("foo.proto"),
        "foo.proto",
        ErrorKind::FileNotIncluded {
            path: dir
                .path()
                .join("include")
                .join("..")
                .join("include")
                .join("foo.proto"),
        },
    );
}

#[test]
fn rel_subdir_include_simple_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success("include", "foo.proto", "foo.proto");
    });
}

#[test]
fn rel_subdir_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success("include", "dir/foo.proto", "dir/foo.proto");
    });
}

#[test]
fn rel_subdir_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            "include",
            Path::new("include").join("foo.proto"),
            "foo.proto",
        );
    });
}

#[test]
fn rel_subdir_include_rel_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            "include",
            Path::new("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    });
}

#[test]
fn rel_subdir_include_abs_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            "include",
            dir.path().join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("foo.proto"),
            },
        );
    });
}

#[test]
fn rel_subdir_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            "include",
            dir.path().join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("dir").join("foo.proto"),
            },
        );
    });
}

#[test]
fn rel_subdir_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            "include",
            Path::new(".").join("include").join("foo.proto"),
            "foo.proto",
        );
    });
}

#[test]
fn rel_subdir_include_dot_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            "include",
            Path::new(".").join("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    });
}

#[test]
fn rel_subdir_include_complex_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            "include",
            Path::new("include")
                .join("..")
                .join("include")
                .join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new("include")
                    .join("..")
                    .join("include")
                    .join("foo.proto"),
            },
        );
    });
}

#[test]
fn dot_include_simple_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(".", "foo.proto", "foo.proto");
    });
}

#[test]
fn dot_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(".", "dir/foo.proto", "dir/foo.proto");
    });
}

#[test]
fn dot_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(".", "foo.proto", "foo.proto");
    });
}

#[test]
fn dot_include_rel_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(".", Path::new("dir").join("foo.proto"), "dir/foo.proto");
    });
}

#[test]
fn dot_include_abs_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            ".",
            dir.path().join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("foo.proto"),
            },
        );
    });
}

#[test]
fn dot_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            ".",
            dir.path().join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("dir").join("foo.proto"),
            },
        );
    });
}

#[test]
fn dot_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(".", Path::new(".").join("foo.proto"), "foo.proto");
    });
}

#[test]
fn dot_include_dot_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            ".",
            Path::new(".").join("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_simple_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(Path::new(".").join("include"), "foo.proto", "foo.proto");
    });
}

#[test]
fn dot_subdir_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include"),
            "dir/foo.proto",
            "dir/foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include"),
            Path::new("include").join("foo.proto"),
            "foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_rel_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include"),
            Path::new("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_abs_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            Path::new(".").join("include"),
            dir.path().join("include").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("include").join("foo.proto"),
            },
        );
    });
}

#[test]
fn dot_subdir_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            Path::new(".").join("include"),
            dir.path().join("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.path().join("include").join("dir").join("foo.proto"),
            },
        );
    });
}

#[test]
fn dot_subdir_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include"),
            Path::new(".").join("include").join("foo.proto"),
            "foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_dot_subdir_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include"),
            Path::new(".").join("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
        );
    });
}

#[test]
fn dot_subdir_include_complex_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            Path::new(".").join("include"),
            Path::new("include")
                .join("..")
                .join("include")
                .join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: Path::new("include")
                    .join("..")
                    .join("include")
                    .join("foo.proto"),
            },
        );
    });
}

#[test]
fn complex_include_complex_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_success(
            Path::new(".").join("include").join("..").join("include"),
            Path::new(".")
                .join("include")
                .join("..")
                .join("include")
                .join("foo.proto"),
            "foo.proto",
        );
    });
}

#[test]
fn invalid_file() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("foo.proto"), INVALID_UTF8).unwrap();

    let mut compiler = Compiler::new(once(&dir)).unwrap();
    let err = compiler.open_file("foo.proto").unwrap_err();

    match err.kind() {
        ErrorKind::FileInvalidUtf8 { name } => {
            assert_eq!(name, "foo.proto");
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn shadow_file_rel() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        std::fs::write("foo.proto", EMPTY).unwrap();
        fs::create_dir_all("include").unwrap();
        std::fs::write(Path::new("include").join("foo.proto"), EMPTY).unwrap();

        let mut compiler = Compiler::new(["include", "."]).unwrap();
        let err = compiler.open_file("foo.proto").unwrap_err();

        match err.kind() {
            ErrorKind::FileShadowed { name, path, shadow } => {
                assert_eq!(name, "foo.proto");
                assert_eq!(path, Path::new("foo.proto"));
                assert_eq!(shadow, &Path::new("include").join("foo.proto"));
            }
            kind => panic!("unexpected error: {}", kind),
        }
    });
}

#[test]
fn shadow_file_rel_subdir() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        fs::create_dir_all("include1").unwrap();
        std::fs::write(Path::new("include1").join("foo.proto"), EMPTY).unwrap();

        fs::create_dir_all("include2").unwrap();
        std::fs::write(Path::new("include2").join("foo.proto"), EMPTY).unwrap();

        let mut compiler = Compiler::new(["include1", "include2"]).unwrap();
        let err = compiler
            .open_file(Path::new("include2").join("foo.proto"))
            .unwrap_err();

        match err.kind() {
            ErrorKind::FileShadowed { name, path, shadow } => {
                assert_eq!(name, "foo.proto");
                assert_eq!(path, &Path::new("include2").join("foo.proto"));
                assert_eq!(shadow, &Path::new("include1").join("foo.proto"));
            }
            kind => panic!("unexpected error: {}", kind),
        }
    });
}

#[test]
fn shadow_file_abs() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("foo.proto"), EMPTY).unwrap();
    fs::create_dir_all(dir.path().join("include")).unwrap();
    std::fs::write(dir.path().join("include").join("foo.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new([dir.path().join("include").as_ref(), dir.path()]).unwrap();
    let err = compiler
        .open_file(dir.path().join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { name, path, shadow } => {
            assert_eq!(name, "foo.proto");
            assert_eq!(path, &dir.path().join("foo.proto"));
            assert_eq!(shadow, &dir.path().join("include").join("foo.proto"));
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn shadow_file_abs_subdir() {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.path().join("include1")).unwrap();
    std::fs::write(dir.path().join("include1").join("foo.proto"), EMPTY).unwrap();

    fs::create_dir_all(dir.path().join("include2")).unwrap();
    std::fs::write(dir.path().join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler =
        Compiler::new([dir.path().join("include1"), dir.path().join("include2")]).unwrap();
    let err = compiler
        .open_file(dir.path().join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { name, path, shadow } => {
            assert_eq!(name, "foo.proto");
            assert_eq!(path, &dir.path().join("include2").join("foo.proto"));
            assert_eq!(shadow, &dir.path().join("include1").join("foo.proto"));
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn shadow_invalid_file() {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.path().join("include1")).unwrap();
    std::fs::write(dir.path().join("include1").join("foo.proto"), INVALID_UTF8).unwrap();

    fs::create_dir_all(dir.path().join("include2")).unwrap();
    std::fs::write(dir.path().join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler =
        Compiler::new([dir.path().join("include1"), dir.path().join("include2")]).unwrap();
    let err = compiler
        .open_file(dir.path().join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileInvalidUtf8 { name } => {
            assert_eq!(name, "foo.proto");
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn shadow_already_imported_file() {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.path().join("include1")).unwrap();
    std::fs::write(dir.path().join("include1").join("foo.proto"), EMPTY).unwrap();

    fs::create_dir_all(dir.path().join("include2")).unwrap();
    std::fs::write(dir.path().join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler =
        Compiler::new([dir.path().join("include1"), dir.path().join("include2")]).unwrap();
    compiler.open_file("foo.proto").unwrap();
    let err = compiler
        .open_file(dir.path().join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { name, path, shadow } => {
            assert_eq!(name, "foo.proto");
            assert_eq!(path, &dir.path().join("include2").join("foo.proto"));
            assert_eq!(shadow, &dir.path().join("include1").join("foo.proto"));
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn import_files() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.path().join("include")).unwrap();
    std::fs::write(
        dir.path().join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(dir.path().join("root.proto"), "import 'dep.proto';").unwrap();
    std::fs::write(dir.path().join("dep2.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned(), dir.path().join("include")]).unwrap();
    compiler.open_file("root.proto").unwrap();

    assert_eq!(compiler.files().len(), 3);

    assert_eq!(compiler.files().next().unwrap().name(), "dep2.proto");
    assert_eq!(
        compiler.files["dep2.proto"].path(),
        Some(dir.path().join("dep2.proto").as_ref())
    );

    assert_eq!(compiler.files().nth(1).unwrap().name(), "dep.proto");
    assert_eq!(
        compiler.files["dep.proto"].path(),
        Some(dir.path().join("include").join("dep.proto").as_ref())
    );

    assert_eq!(compiler.files().nth(2).unwrap().name(), "root.proto");
    assert_eq!(
        compiler.files["root.proto"].path(),
        Some(dir.path().join("root.proto").as_ref())
    );

    let file_descriptor_set = compiler.file_descriptor_set();
    assert_eq!(file_descriptor_set.file.len(), 1);
    assert_eq!(file_descriptor_set.file[0].name(), "root.proto");

    compiler.include_imports(true);
    let file_descriptor_set = compiler.file_descriptor_set();
    assert_eq!(file_descriptor_set.file.len(), 3);
    assert_eq!(file_descriptor_set.file[0].name(), "dep2.proto");
    assert_eq!(file_descriptor_set.file[1].name(), "dep.proto");
    assert_eq!(file_descriptor_set.file[2].name(), "root.proto");
}

#[test]
fn import_files_include_imports_path_already_imported() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("root1.proto"), "import 'root2.proto';").unwrap();
    std::fs::write(dir.path().join("root2.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned()]).unwrap();
    compiler.open_file("root1.proto").unwrap();

    let file_descriptor_set = compiler.file_descriptor_set();
    assert_eq!(file_descriptor_set.file.len(), 1);
    assert_eq!(file_descriptor_set.file[0].name(), "root1.proto");

    compiler.open_file("root2.proto").unwrap();

    let file_descriptor_set = compiler.file_descriptor_set();
    assert_eq!(file_descriptor_set.file.len(), 2);
    assert_eq!(file_descriptor_set.file[0].name(), "root2.proto");
    assert_eq!(file_descriptor_set.file[1].name(), "root1.proto");
}

#[test]
fn import_cycle() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.path().join("include")).unwrap();
    std::fs::write(
        dir.path().join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(dir.path().join("root.proto"), "import 'dep.proto';").unwrap();
    std::fs::write(dir.path().join("dep2.proto"), "import 'root.proto';").unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned(), dir.path().join("include")]).unwrap();
    let err = compiler.open_file("root.proto").unwrap_err();

    match err.kind() {
        ErrorKind::CircularImport { name, cycle } => {
            assert_eq!(name, "root.proto");
            assert_eq!(cycle, "root.proto -> dep.proto -> dep2.proto -> root.proto")
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn import_cycle_short() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("root.proto"), "import 'dep.proto';").unwrap();
    std::fs::write(dir.path().join("dep.proto"), "import 'dep.proto';").unwrap();

    let mut compiler = Compiler::new([dir.path()]).unwrap();
    let err = compiler.open_file("root.proto").unwrap_err();

    match err.kind() {
        ErrorKind::CircularImport { name, cycle } => {
            assert_eq!(name, "dep.proto");
            assert_eq!(cycle, "root.proto -> dep.proto -> dep.proto")
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn import_cycle_nested() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.path().join("root.proto"), "import 'root.proto';").unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned(), dir.path().join("include")]).unwrap();
    let err = compiler.open_file("root.proto").unwrap_err();

    match err.kind() {
        ErrorKind::CircularImport { name, cycle } => {
            assert_eq!(name, "root.proto");
            assert_eq!(cycle, "root.proto -> root.proto")
        }
        kind => panic!("unexpected error: {}", kind),
    }
}

#[test]
fn duplicated_import() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.path().join("include")).unwrap();
    std::fs::write(
        dir.path().join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(
        dir.path().join("root.proto"),
        "import 'dep.proto'; import 'dep2.proto';",
    )
    .unwrap();
    std::fs::write(dir.path().join("dep2.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned(), dir.path().join("include")]).unwrap();
    compiler.open_file("root.proto").unwrap();

    assert_eq!(compiler.files().len(), 3);

    assert_eq!(compiler.files().next().unwrap().name(), "dep2.proto");
    assert_eq!(
        compiler.files["dep2.proto"].path(),
        Some(dir.path().join("dep2.proto").as_ref())
    );

    assert_eq!(compiler.files().nth(1).unwrap().name(), "dep.proto");
    assert_eq!(
        compiler.files["dep.proto"].path(),
        Some(dir.path().join("include").join("dep.proto").as_ref())
    );

    assert_eq!(compiler.files().nth(2).unwrap().name(), "root.proto");
    assert_eq!(
        compiler.files["root.proto"].path(),
        Some(dir.path().join("root.proto").as_ref())
    );
}

#[test]
fn import_file_absolute_path() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.path().join("include")).unwrap();
    std::fs::write(dir.path().join("include").join("dep.proto"), EMPTY).unwrap();

    std::fs::write(
        dir.path().join("root.proto"),
        format!(
            "import '{}';",
            dir.path()
                .join("include")
                .join("dep.proto")
                .display()
                .to_string()
                .replace('\\', "/")
                .escape_default()
        ),
    )
    .unwrap();

    let mut compiler = Compiler::new([dir.path().to_owned(), dir.path().join("include")]).unwrap();
    compiler.open_file("root.proto").unwrap_err();
}

#[cfg(windows)]
#[test]
fn open_file_case_insensitive() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.path().join("include"),
        dir.path().join("INCLUDE").join("foo.proto"),
        "foo.proto",
    );
}
