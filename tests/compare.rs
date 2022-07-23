use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use assert_fs::TempDir;
use prost::Message;
use prost_reflect::{ReflectMessage, SerializeOptions};
use prost_types::FileDescriptorSet;
use similar_asserts::assert_serde_eq;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("tests/data")
}

fn google_proto_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("protobuf/src/google/protobuf")
}

fn compare(name: &str) {
    let expected = protoc(name);
    let actual = protox(name);

    // std::fs::write("expected.yml", &expected);
    // std::fs::write("actual.yml", &actual);

    assert_serde_eq!(actual, expected);
}

fn protoc(name: &str) -> String {
    let tempdir = TempDir::new().unwrap();
    let result = tempdir.join("desc.bin");
    let output = Command::new(prost_build::protoc())
        .arg("--proto_path")
        .arg(test_data_dir())
        .arg("--proto_path")
        .arg(google_proto_dir())
        .arg("--include_imports")
        .arg("--include_source_info")
        .arg(format!("--descriptor_set_out={}", result.display()))
        .arg(format!("{}.proto", name))
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!(
            "protoc did not succeed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let bytes = fs::read(result).unwrap();

    let descriptor = FileDescriptorSet::decode(bytes.as_ref()).unwrap();

    file_descriptor_to_yaml(descriptor)
}

fn protox(name: &str) -> String {
    let descriptor = protox::compile(&[format!("{}.proto", name)], &[test_data_dir()]).unwrap();
    file_descriptor_to_yaml(descriptor)
}

fn file_descriptor_to_yaml(mut descriptor: FileDescriptorSet) -> String {
    for file in &mut descriptor.file {
        // Normalize ordering of spans
        if let Some(source_code_info) = &mut file.source_code_info {
            source_code_info
                .location
                .sort_by(|l, r| l.span.cmp(&r.span).then_with(|| l.path.cmp(&r.path)));
        }
    }

    let message = descriptor.transcode_to_dynamic();
    let mut serializer = serde_yaml::Serializer::new(Vec::new());
    message
        .serialize_with_options(
            &mut serializer,
            &SerializeOptions::new()
                .skip_default_fields(true)
                .stringify_64_bit_integers(false),
        )
        .unwrap();
    String::from_utf8(serializer.into_inner()).unwrap()
}

macro_rules! compare {
    ($name:ident) => {
        #[test]
        fn $name() {
            compare(stringify!($name));
        }
    };
}

compare!(empty_file);
compare!(empty_file_with_comment);
compare!(field_defaults);
compare!(generate_map_entry_message);
compare!(generate_group_message);
compare!(generate_synthetic_oneof_ordering);
compare!(generate_synthetic_oneof);
compare!(generated_message_ordering);
compare!(multiple_extends);
compare!(name_resolution);
compare!(option_merge_message);
compare!(custom_json_name);
// TODO need to get protoc to use the exact same versions of these files
// compare!(import_google);

// TODO borrow some test protos from protobuf repository
