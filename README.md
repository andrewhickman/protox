[![Crates.io][ci]][cl] [![Docs.rs][di]][dl]

[ci]: https://img.shields.io/crates/v/protox.svg
[cl]: https://crates.io/crates/protox/

[di]: https://docs.rs/protox/badge.svg
[dl]: https://docs.rs/protox/

# protox

  ------------------------ FileResolver ---
  | parse                                 |
  |   |                                   |
  | build "ir" (FileDescriptorProto)      |
  |   |                                   |
  -----------------------------------------
      |
    get names (NameMap / DescriptorMap)
      |
    check imports
      |
    check FileDescriptorProto + mutate to resolve names etc

