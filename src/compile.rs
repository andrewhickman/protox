use std::{
    fs, io,
    path::{self, Component, Path, PathBuf},
    sync::Arc,
};

use miette::NamedSource;
use prost_types::FileDescriptorSet;

use crate::{ast, parse, Error, ErrorKind};

#[derive(Debug)]
pub struct Compiler {
    includes: Vec<PathBuf>,
    files: Vec<File>,
}

#[derive(Debug)]
struct File {
    ast: ast::File,
    src: NamedSource,
    path: PathBuf,
    name: String,
}

#[derive(Debug)]
enum ImportResult<'a> {
    Found {
        include: &'a Path,
        path: PathBuf,
        source: Arc<str>,
    },
    AlreadyImported {
        include: &'a Path,
        path: PathBuf,
        // ast: &'a ast::File,
    },
    NotFound,
    OpenError {
        include: &'a Path,
        path: PathBuf,
        err: io::Error,
    },
}

impl Compiler {
    pub fn new(includes: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<Self, Error> {
        let includes: Vec<_> = includes
            .into_iter()
            .map(|path| path.as_ref().to_owned())
            .collect();
        if includes.is_empty() {
            return Err(Error::new(ErrorKind::NoIncludePaths));
        }
        Ok(Compiler {
            includes,
            files: Vec::new(),
        })
    }

    pub fn add_file(&mut self, relative_path: impl AsRef<Path>) -> Result<(), Error> {
        let relative_path = relative_path.as_ref();
        let (resolved_include, name) =
            self.resolve_import_name(relative_path).ok_or_else(|| {
                Error::new(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                })
            })?;

        let (source, path) = match self.resolve_import(&name) {
            ImportResult::Found {
                include,
                path,
                source,
            } => {
                if resolved_include.is_some() && resolved_include != Some(include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    (source, path)
                }
            }
            ImportResult::AlreadyImported { include, path } => {
                if resolved_include.is_some() && resolved_include != Some(include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    return Ok(());
                }
            }
            ImportResult::NotFound => {
                return Err(Error::new(ErrorKind::FileNotIncluded {
                    path: relative_path.to_owned(),
                }))
            }
            ImportResult::OpenError { include, path, err } => {
                if resolved_include.is_some() && resolved_include != Some(include) {
                    return Err(Error::new(ErrorKind::FileShadowed {
                        path: relative_path.to_owned(),
                        shadow: path,
                    }));
                } else {
                    return Err(Error::new(ErrorKind::OpenFile { path, err }));
                }
            }
        };

        let src = NamedSource::new(&name, source.clone());
        let ast = match parse::parse(&source) {
            Ok(ast) => ast,
            Err(errors) => return Err(Error::new(ErrorKind::ParseErrors { src, errors })),
        };

        self.files.push(File {
            ast,
            src,
            name,
            path,
        });

        Ok(())
    }

    pub fn build_file_descriptor_set(self) -> FileDescriptorSet {
        FileDescriptorSet::default()
        // todo!()
    }

    fn resolve_import_name(&self, path: &Path) -> Option<(Option<&Path>, String)> {
        for include in &self.includes {
            if let Some(relative_path) = strip_prefix(path, include) {
                if let Some(import_name) = get_import_name(relative_path) {
                    return Some((Some(include), import_name));
                } else {
                    continue;
                }
            }
        }

        get_import_name(path).map(|import_name| (None, import_name))
    }

    fn resolve_import(&self, name: &str) -> ImportResult {
        for include in &self.includes {
            let candidate_path = include.join(name);
            match fs::read_to_string(&candidate_path) {
                Ok(source) => {
                    return ImportResult::Found {
                        include,
                        path: candidate_path,
                        source: source.into(),
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return ImportResult::OpenError {
                        include,
                        path: candidate_path,
                        err,
                    };
                }
            }
        }

        ImportResult::NotFound
    }
}

fn get_import_name(path: &Path) -> Option<String> {
    let mut name = String::new();
    for component in path.components() {
        match component {
            path::Component::Normal(component) => {
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

/// Modification of std::path::Path::strip_prefix which ignores '.' components
fn strip_prefix<'a>(path: &'a Path, prefix: &Path) -> Option<&'a Path> {
    let mut path = path.components();
    let mut prefix = prefix.components();

    loop {
        let mut path_next = path.clone();
        let mut prefix_next = prefix.clone();

        match (path_next.next(), prefix_next.next()) {
            (Some(Component::CurDir), _) => {
                path = path_next;
            }
            (_, Some(Component::CurDir)) => {
                prefix = prefix_next;
            }
            (Some(ref x), Some(ref y)) if x == y => {
                path = path_next;
                prefix = prefix_next;
            }
            (Some(_), Some(_)) => return None,
            (Some(_), None) => return Some(path.as_path()),
            (None, None) => return Some(path.as_path()),
            (None, Some(_)) => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::iter::{once, empty};

    use assert_fs::TempDir;

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

        assert_eq!(compiler.files.len(), 1);
        assert_eq!(compiler.files[0].ast, ast::File::default());
        assert_eq!(compiler.files[0].name.as_str(), name);
        assert_eq!(compiler.files[0].path, include.join(name));
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

        match (err.kind, expected_err) {
            (
                ErrorKind::FileNotIncluded { path: lpath },
                ErrorKind::FileNotIncluded { path: rpath },
            ) => assert_eq!(lpath, rpath),
            (err, _) => panic!("unexpected error: {}", err),
        }
        assert_eq!(compiler.files.len(), 0);
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
        let err = Compiler::new(empty::<PathBuf>()).unwrap_err();
        match err.kind {
            ErrorKind::NoIncludePaths => (),
            kind => panic!("unexpected error {}", kind),
        }
    }

    #[test]
    #[ignore]
    fn invalid_file() {
        todo!()
    }

    #[test]
    #[ignore]
    fn shadow_file() {
        todo!()
    }

    #[test]
    #[ignore]
    fn shadow_invalid_file() {

    }

    #[test]
    #[ignore]
    fn shadow_already_imported_file() {
        todo!()
    }
}
