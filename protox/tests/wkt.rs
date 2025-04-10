use prost_reflect::ReflectMessage;
use prost_types::FileDescriptorSet;
use protox::{file::GoogleFileResolver, Compiler};

#[test]
fn prost_reflect_wkt_matches_compiled_wkt() {
    use prost::Message;

    let desc = FileDescriptorSet::decode(expected_well_known_types().as_slice()).unwrap();
    let prost_wkt_desc = FileDescriptorSet {
        file: ().descriptor().parent_pool().file_descriptor_protos().cloned().collect(),
    };

    if desc != prost_wkt_desc {
        let actual = format!("{prost_wkt_desc:#?}");
        let expected = format!("{desc:#?}");
        let diff = similar_asserts::SimpleDiff::from_str(&actual, &expected, "actual", "expected");

        // If this fails with a non-trivial diff it's reasonable to just dump `desc` via
        // the debug representation and afterwards adjust the output to be valid rust code
        // The following steps were done for the intial version:
        //
        // * replace `[` with `vec![`
        // * Call `.into()` on strings and enum variants
        // * Wrap all options field into `Options::from_prost`
        //
        // The first two steps can be easily done with a bunch of search and
        // replace queries for almost all instances. There are a few cases
        // that need to be manually adjusted afterwards
        //
        // The last step requires manually going through these fields, but
        // that's only ~10 instances.
        panic!(
            "The well known file descriptor returned by `make_description()` \
             does not match the expected file descriptor parsed from `src/well_known_types_bytes.bin`: \
             {diff}"
        );
    }
}

fn expected_well_known_types() -> Vec<u8> {
    // protox can output a FileDescriptorSet directly, but by going through bytes, this should still work
    // when upgrading to a newer prost-types version.
    Compiler::with_file_resolver(GoogleFileResolver::new())
        .include_source_info(false)
        .open_files([
            "google/protobuf/any.proto",
            "google/protobuf/api.proto",
            "google/protobuf/descriptor.proto",
            "google/protobuf/duration.proto",
            "google/protobuf/empty.proto",
            "google/protobuf/field_mask.proto",
            "google/protobuf/source_context.proto",
            "google/protobuf/struct.proto",
            "google/protobuf/timestamp.proto",
            "google/protobuf/type.proto",
            "google/protobuf/wrappers.proto",
            "google/protobuf/compiler/plugin.proto",
        ])
        .unwrap()
        .encode_file_descriptor_set()
}
