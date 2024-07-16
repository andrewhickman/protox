use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use prost_reflect::{DescriptorPool, DynamicMessage, SerializeOptions, Value};
use prost_types::{field_descriptor_proto::Type, source_code_info::Location};
use tempfile::TempDir;

fn test_data_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("tests/data")
}

fn google_proto_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("protobuf/src/google/protobuf")
}

fn google_src_dir() -> PathBuf {
    PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("protobuf/src")
}

fn compare(name: &str) {
    let files = if name == "descriptor" {
        vec![format!("{}.proto", name)]
    } else {
        vec![
            // Ensure we use a consistent version of descriptor.proto
            "google/protobuf/descriptor.proto".to_owned(),
            format!("{}.proto", name),
        ]
    };

    let expected = to_yaml(&protoc(&files));
    let actual = to_yaml(&protox(&files));

    similar_asserts::assert_eq!(expected, actual);
}

fn to_yaml(message: &DynamicMessage) -> String {
    let mut serializer = serde_yaml::Serializer::new(Vec::new());
    message
        .serialize_with_options(
            &mut serializer,
            &SerializeOptions::new()
                .skip_default_fields(true)
                .stringify_64_bit_integers(false),
        )
        .unwrap();
    String::from_utf8(serializer.into_inner().unwrap()).unwrap()
}

fn protoc(files: &[String]) -> DynamicMessage {
    let tempdir = TempDir::new().unwrap();
    let result = tempdir.path().join("desc.bin");
    let output = Command::new(prost_build::protoc_from_env())
        .arg("--proto_path")
        .arg(test_data_dir())
        .arg("--proto_path")
        .arg(google_proto_dir())
        .arg("--proto_path")
        .arg(google_src_dir())
        .arg("--include_imports")
        .arg("--include_source_info")
        .arg(format!("--descriptor_set_out={}", result.display()))
        .args(files)
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

    decode_file_descriptor(bytes)
}

fn protox(files: &[String]) -> DynamicMessage {
    let descriptor = protox::Compiler::new([test_data_dir(), google_proto_dir(), google_src_dir()])
        .unwrap()
        .include_imports(true)
        .include_source_info(true)
        .open_files(files)
        .unwrap()
        .encode_file_descriptor_set();
    decode_file_descriptor(descriptor)
}

fn decode_file_descriptor(bytes: Vec<u8>) -> DynamicMessage {
    let pool = DescriptorPool::decode(bytes.as_slice()).unwrap();
    let desc = pool
        .get_message_by_name("google.protobuf.FileDescriptorSet")
        .unwrap();
    let mut file_set = DynamicMessage::decode(desc, bytes.as_slice()).unwrap();

    let files = file_set
        .get_field_by_name_mut("file")
        .unwrap()
        .as_list_mut()
        .unwrap();

    // We can't compare google.protobuf files directly since they are baked into protoc and may be a different version to
    // what we are using. (The google_protobuf_* tests ensures we are compiling these files correctly)
    files.retain(|f| {
        !f.as_message()
            .unwrap()
            .get_field_by_name("name")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("google/protobuf/")
    });
    debug_assert!(!files.is_empty());

    for file in files {
        let file = file.as_message_mut().unwrap();

        // Normalize ordering of spans
        let locations = file
            .get_field_by_name_mut("source_code_info")
            .unwrap()
            .as_message_mut()
            .unwrap()
            .get_field_by_name_mut("location")
            .unwrap()
            .as_list_mut()
            .unwrap();
        locations.sort_unstable_by_key(|location| {
            let location = location
                .as_message()
                .unwrap()
                .transcode_to::<Location>()
                .unwrap();
            (location.path, location.span)
        });

        // Our formatting of floats is slightly different to protoc (and exact conformance is tricky), so we normalize
        // them in default values
        visit_messages(
            file.get_field_by_name_mut("message_type")
                .unwrap()
                .as_list_mut()
                .unwrap(),
            &|message| {
                for field in message
                    .get_field_by_name_mut("field")
                    .unwrap()
                    .as_list_mut()
                    .unwrap()
                {
                    let field = field.as_message_mut().unwrap();
                    let ty = field
                        .get_field_by_name("type")
                        .unwrap()
                        .as_enum_number()
                        .unwrap();
                    let default_value = field
                        .get_field_by_name_mut("default_value")
                        .unwrap()
                        .as_string_mut()
                        .unwrap();
                    if !default_value.is_empty()
                        && matches!(Type::try_from(ty), Ok(Type::Float | Type::Double))
                    {
                        *default_value = default_value.parse::<f64>().unwrap().to_string();
                    }
                }
            },
        )
    }

    file_set
}

fn visit_messages(messages: &mut [Value], f: &impl Fn(&mut DynamicMessage)) {
    for message in messages {
        let message = message.as_message_mut().unwrap();
        f(message);
        visit_messages(
            message
                .get_field_by_name_mut("nested_type")
                .unwrap()
                .as_list_mut()
                .unwrap(),
            f,
        );
    }
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
compare!(reserved_ranges);
compare!(oneof_group_field);
compare!(service);
compare!(option_group_field);
compare!(message_name_field_name_conflict);

#[test]
fn google_protobuf_any() {
    compare("any");
}
#[test]

fn google_protobuf_api() {
    compare("api");
}

#[test]
fn google_protobuf_descriptor() {
    compare("descriptor");
}

#[test]
fn google_protobuf_duration() {
    compare("duration");
}

#[test]
fn google_protobuf_empty() {
    compare("empty");
}

#[test]
fn google_protobuf_field_mask() {
    compare("field_mask");
}

#[test]
fn google_protobuf_source_context() {
    compare("source_context");
}

#[test]
fn google_protobuf_struct() {
    compare("struct");
}

#[test]
fn google_protobuf_timestamp() {
    compare("timestamp");
}

#[test]
fn google_protobuf_type() {
    compare("type");
}

#[test]
fn google_protobuf_wrappers() {
    compare("wrappers");
}

#[test]
fn google_protobuf_compiler_plugin() {
    compare("compiler/plugin");
}

#[test]
fn google_map_proto2_unittest() {
    compare("map_proto2_unittest");
}

#[test]
fn google_map_unittest() {
    compare("map_unittest");
}

#[test]
fn google_test_messages_proto2() {
    compare("test_messages_proto2");
}

#[test]
fn google_test_messages_proto3() {
    compare("test_messages_proto3");
}

#[test]
fn google_unittest_custom_options() {
    compare("unittest_custom_options");
}

#[test]
fn google_unittest_empty() {
    compare("unittest_empty");
}

#[test]
fn google_unittest_enormous_descriptor() {
    compare("unittest_enormous_descriptor");
}

#[test]
fn google_unittest_import() {
    compare("unittest_import");
}

#[test]
fn google_unittest_lazy_dependencies() {
    compare("unittest_lazy_dependencies");
}

#[test]
fn google_unittest_no_field_presence() {
    compare("unittest_no_field_presence");
}

#[test]
fn google_unittest_preserve_unknown_enum() {
    compare("unittest_preserve_unknown_enum");
}

#[test]
fn google_unittest_preserve_unknown_enum2() {
    compare("unittest_preserve_unknown_enum2");
}

#[test]
fn google_unittest_proto3_optional() {
    compare("unittest_proto3_optional");
}

#[test]
fn google_unittest_proto3() {
    compare("unittest_proto3");
}

#[test]
fn google_unittest_well_known_types() {
    compare("unittest_well_known_types");
}

#[test]
fn google_unittest() {
    compare("unittest");
}
