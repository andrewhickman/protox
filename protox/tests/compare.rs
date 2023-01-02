use std::{
    env, fs,
    path::PathBuf,
    process::{Command, Stdio},
};

use assert_fs::TempDir;
use prost::Message;
use prost_reflect::{DynamicMessage, ReflectMessage, SerializeOptions};
use prost_types::{field_descriptor_proto::Type, DescriptorProto, FileDescriptorSet};
use similar_asserts::assert_serde_eq;

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
    let expected = protoc(name);
    let actual = protox(name);

    // std::fs::write("expected.yml", to_yaml(&expected));
    // std::fs::write("actual.yml", to_yaml(&actual));

    assert_serde_eq!(expected, actual);
}

#[allow(unused)]
fn to_yaml(message: &DynamicMessage) -> Vec<u8> {
    let mut serializer = serde_yaml::Serializer::new(Vec::new());
    message
        .serialize_with_options(
            &mut serializer,
            &SerializeOptions::new()
                .skip_default_fields(true)
                .stringify_64_bit_integers(false),
        )
        .unwrap();
    serializer.into_inner()
}

fn protoc(name: &str) -> DynamicMessage {
    let tempdir = TempDir::new().unwrap();
    let result = tempdir.join("desc.bin");
    let output = Command::new(prost_build::protoc())
        .arg("--proto_path")
        .arg(test_data_dir())
        .arg("--proto_path")
        .arg(google_proto_dir())
        .arg("--proto_path")
        .arg(google_src_dir())
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

    file_descriptor_to_dynamic(descriptor)
}

fn protox(name: &str) -> DynamicMessage {
    let descriptor = protox::compile(
        [format!("{}.proto", name)],
        [test_data_dir(), google_proto_dir(), google_src_dir()],
    )
    .unwrap();
    file_descriptor_to_dynamic(descriptor)
}

fn file_descriptor_to_dynamic(mut descriptor: FileDescriptorSet) -> DynamicMessage {
    for file in &mut descriptor.file {
        // Normalize ordering of spans
        if let Some(source_code_info) = &mut file.source_code_info {
            source_code_info
                .location
                .sort_unstable_by(|l, r| l.path.cmp(&r.path).then_with(|| l.span.cmp(&r.span)));
        }

        // Our formatting of floats is slightly different to protoc (and exact conformance is tricky), so we normalize
        // them in default values
        visit_messages(&mut file.message_type, &|message| {
            for field in &mut message.field {
                if !field.default_value().is_empty()
                    && matches!(field.r#type(), Type::Float | Type::Double)
                {
                    field.default_value =
                        Some(field.default_value().parse::<f64>().unwrap().to_string());
                }
            }
        })
    }

    // We can't compare google.protobuf files directly since they are baked into protoc and may be a different version to
    // what we are using. (The google_protobuf_* tests ensures we are compiling these files correctly)
    descriptor
        .file
        .retain(|f| !f.name().starts_with("google/protobuf/"));
    debug_assert!(!descriptor.file.is_empty());

    descriptor.transcode_to_dynamic()
}

fn visit_messages(messages: &mut [DescriptorProto], f: &impl Fn(&mut DescriptorProto)) {
    for message in messages {
        f(message);
        visit_messages(&mut message.nested_type, f);
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
#[ignore]
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
#[ignore]
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
#[ignore]
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
