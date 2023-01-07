use std::path::{Path, PathBuf};

use crate::file::FileResolver;

use super::IncludeFileResolver;

#[test]
fn test_resolve_path() {
    let include = IncludeFileResolver::new("/path/to/include".into());

    #[cfg(unix)]
    fn non_utf8_path() -> PathBuf {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

        OsStr::from_bytes([0, 159, 146, 150]).into()
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
