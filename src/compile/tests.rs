use std::{
    fs, io,
    iter::{empty, once},
};

use assert_fs::TempDir;
use prost_types::FileDescriptorProto;

use super::*;
use crate::with_current_dir;

const EMPTY: &[u8] = &[];
const INVALID_UTF8: &[u8] = &[255];

fn test_compile_success(include: impl AsRef<Path>, file: impl AsRef<Path>, name: &str) {
    let include = include.as_ref();
    let file = file.as_ref();

    std::fs::create_dir_all(include).unwrap();
    if let Some(parent) = include.join(name).parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(include.join(name), EMPTY).unwrap();

    let mut compiler = Compiler::new(once(include)).unwrap();
    compiler.add_file(file).unwrap();

    assert_eq!(compiler.file_map.iter().count(), 1);
    // TODO name
    assert_eq!(compiler.file_map[name].name, name);
    assert_eq!(
        compiler.file_map[name].descriptor,
        FileDescriptorProto {
            name: Some(name.to_owned()),
            ..Default::default()
        }
    );
    assert_eq!(compiler.file_map[name].path, include.join(name));
    assert_eq!(compiler.file_map[name].include, include);
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
    let err = compiler.add_file(file).unwrap_err();

    match (err.kind(), &expected_err) {
        (
            ErrorKind::FileNotIncluded { path: lpath },
            ErrorKind::FileNotIncluded { path: rpath },
        ) => assert_eq!(lpath, rpath),
        (err, _) => panic!("unexpected error: {}", err),
    }
    assert_eq!(compiler.file_map.iter().count(), 0);
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
    test_compile_success(dir.path(), dir.join("foo.proto"), "foo.proto");
}

#[test]
fn abs_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.path(),
        dir.join("dir").join("foo.proto"),
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
    test_compile_success(dir.join("include"), "foo.proto", "foo.proto");
}

#[test]
fn abs_subdir_include_simple_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(dir.join("include"), "dir/foo.proto", "dir/foo.proto");
}

#[test]
fn abs_subdir_include_rel_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.join("include"),
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
            dir.join("include"),
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
    test_compile_success(&dir, dir.join("foo.proto"), "foo.proto");
}

#[test]
fn abs_subdir_include_abs_subdir_file() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.join("include"),
        dir.join("include").join("foo.proto"),
        "foo.proto",
    );
}

#[test]
fn abs_subdir_include_dot_file() {
    let dir = TempDir::new().unwrap();
    with_current_dir(&dir, || {
        test_compile_error(
            dir.join("include"),
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
            dir.join("include"),
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
        dir.join("dir").join("..").join("dir").join("foo.proto"),
        "dir/foo.proto",
        ErrorKind::FileNotIncluded {
            path: dir.join("dir").join("..").join("dir").join("foo.proto"),
        },
    );
}

#[test]
fn abs_subdir_include_complex_file() {
    let dir = TempDir::new().unwrap();
    test_compile_error(
        dir.join("include"),
        dir.join("include")
            .join("..")
            .join("include")
            .join("foo.proto"),
        "foo.proto",
        ErrorKind::FileNotIncluded {
            path: dir
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
            dir.join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("foo.proto"),
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
            dir.join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("dir").join("foo.proto"),
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
            dir.join("foo.proto"),
            "foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("foo.proto"),
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
            dir.join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("dir").join("foo.proto"),
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
            dir.join("include").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("include").join("foo.proto"),
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
            dir.join("include").join("dir").join("foo.proto"),
            "dir/foo.proto",
            ErrorKind::FileNotIncluded {
                path: dir.join("include").join("dir").join("foo.proto"),
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
fn no_include_paths() {
    let err = Compiler::new(empty::<std::path::PathBuf>()).unwrap_err();
    match err.kind() {
        ErrorKind::NoIncludePaths => (),
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn invalid_file() {
    let dir = TempDir::new().unwrap().into_persistent();

    std::fs::write(dir.join("foo.proto"), INVALID_UTF8).unwrap();

    let mut compiler = Compiler::new(once(&dir)).unwrap();
    let err = compiler.add_file("foo.proto").unwrap_err();

    match err.kind() {
        ErrorKind::OpenFile { path, err } => {
            assert_eq!(path, &dir.join("foo.proto"));
            assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        }
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn shadow_file() {
    let dir = TempDir::new().unwrap().into_persistent();

    fs::create_dir_all(dir.join("include1")).unwrap();
    std::fs::write(dir.join("include1").join("foo.proto"), EMPTY).unwrap();

    fs::create_dir_all(dir.join("include2")).unwrap();
    std::fs::write(dir.join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new(&[dir.join("include1"), dir.join("include2")]).unwrap();
    let err = compiler
        .add_file(dir.join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { path, shadow } => {
            assert_eq!(path, &dir.join("include2").join("foo.proto"));
            assert_eq!(shadow, &dir.join("include1").join("foo.proto"));
        }
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn shadow_invalid_file() {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.join("include1")).unwrap();
    std::fs::write(dir.join("include1").join("foo.proto"), INVALID_UTF8).unwrap();

    fs::create_dir_all(dir.join("include2")).unwrap();
    std::fs::write(dir.join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new(&[dir.join("include1"), dir.join("include2")]).unwrap();
    let err = compiler
        .add_file(dir.join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { path, shadow } => {
            assert_eq!(path, &dir.join("include2").join("foo.proto"));
            assert_eq!(shadow, &dir.join("include1").join("foo.proto"));
        }
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn shadow_already_imported_file() {
    let dir = TempDir::new().unwrap();

    fs::create_dir_all(dir.join("include1")).unwrap();
    std::fs::write(dir.join("include1").join("foo.proto"), EMPTY).unwrap();

    fs::create_dir_all(dir.join("include2")).unwrap();
    std::fs::write(dir.join("include2").join("foo.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new(&[dir.join("include1"), dir.join("include2")]).unwrap();
    compiler.add_file("foo.proto").unwrap();
    let err = compiler
        .add_file(dir.join("include2").join("foo.proto"))
        .unwrap_err();

    match err.kind() {
        ErrorKind::FileShadowed { path, shadow } => {
            assert_eq!(path, &dir.join("include2").join("foo.proto"));
            assert_eq!(shadow, &dir.join("include1").join("foo.proto"));
        }
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn import_files() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.join("include")).unwrap();
    std::fs::write(
        dir.join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(dir.join("root.proto"), "import 'dep.proto';").unwrap();
    std::fs::write(dir.join("dep2.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new(&[dir.to_path_buf(), dir.join("include")]).unwrap();
    compiler.add_file("root.proto").unwrap();

    assert_eq!(compiler.file_map.iter().count(), 3);

    assert_eq!(compiler.file_map[0].name.as_str(), "dep2.proto");
    assert_eq!(compiler.file_map[0].path, dir.join("dep2.proto"));
    assert_eq!(compiler.file_map[0].include, dir.path());

    assert_eq!(compiler.file_map[1].name.as_str(), "dep.proto");
    assert_eq!(
        compiler.file_map[1].path,
        dir.join("include").join("dep.proto")
    );
    assert_eq!(compiler.file_map[1].include, dir.join("include"));

    assert_eq!(compiler.file_map[2].name.as_str(), "root.proto");
    assert_eq!(compiler.file_map[2].path, dir.join("root.proto"));
    assert_eq!(compiler.file_map[2].include, dir.path());
}

#[test]
fn import_cycle() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.join("include")).unwrap();
    std::fs::write(
        dir.join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(dir.join("root.proto"), "import 'dep.proto';").unwrap();
    std::fs::write(dir.join("dep2.proto"), "import 'root.proto';").unwrap();

    let mut compiler = Compiler::new(&[dir.to_path_buf(), dir.join("include")]).unwrap();
    let err = compiler.add_file("root.proto").unwrap_err();

    match err.kind() {
        ErrorKind::CircularImport { cycle } => {
            assert_eq!(cycle, "root.proto -> dep.proto -> dep2.proto -> root.proto")
        }
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn import_cycle_short() {
    let dir = TempDir::new().unwrap();

    std::fs::write(dir.join("root.proto"), "import 'root.proto';").unwrap();

    let mut compiler = Compiler::new(&[dir.to_path_buf(), dir.join("include")]).unwrap();
    let err = compiler.add_file("root.proto").unwrap_err();

    match err.kind() {
        ErrorKind::CircularImport { cycle } => assert_eq!(cycle, "root.proto -> root.proto"),
        kind => panic!("unexpected error {}", kind),
    }
}

#[test]
fn duplicated_import() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.join("include")).unwrap();
    std::fs::write(
        dir.join("include").join("dep.proto"),
        "import 'dep2.proto';",
    )
    .unwrap();

    std::fs::write(
        dir.join("root.proto"),
        "import 'dep.proto'; import 'dep2.proto';",
    )
    .unwrap();
    std::fs::write(dir.join("dep2.proto"), EMPTY).unwrap();

    let mut compiler = Compiler::new(&[dir.to_path_buf(), dir.join("include")]).unwrap();
    compiler.add_file("root.proto").unwrap();

    assert_eq!(compiler.file_map.iter().count(), 3);

    assert_eq!(compiler.file_map[0].name.as_str(), "dep2.proto");
    assert_eq!(compiler.file_map[0].path, dir.join("dep2.proto"));
    assert_eq!(compiler.file_map[0].include, dir.path());

    assert_eq!(compiler.file_map[1].name.as_str(), "dep.proto");
    assert_eq!(
        compiler.file_map[1].path,
        dir.join("include").join("dep.proto")
    );
    assert_eq!(compiler.file_map[1].include, dir.join("include"));

    assert_eq!(compiler.file_map[2].name.as_str(), "root.proto");
    assert_eq!(compiler.file_map[2].path, dir.join("root.proto"));
    assert_eq!(compiler.file_map[2].include, dir.path());
}

#[test]
fn import_file_absolute_path() {
    let dir = TempDir::new().unwrap();

    fs::create_dir(dir.join("include")).unwrap();
    std::fs::write(dir.join("include").join("dep.proto"), EMPTY).unwrap();

    std::fs::write(
        dir.join("root.proto"),
        dbg!(format!(
            "import '{}';",
            dir.join("include")
                .join("dep.proto")
                .display()
                .to_string()
                .replace('\\', "/")
                .escape_default()
        )),
    )
    .unwrap();

    let mut compiler = Compiler::new(&[dir.to_path_buf(), dir.join("include")]).unwrap();
    compiler.add_file("root.proto").unwrap_err();
}

#[cfg(windows)]
#[test]
fn add_file_case_insensitive() {
    let dir = TempDir::new().unwrap();
    test_compile_success(
        dir.join("include"),
        dir.join("INCLUDE").join("foo.proto"),
        "foo.proto",
    );
}
