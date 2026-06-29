const UNSUPPORTED_REQUEST_CONTENT_TYPES: &[&str] = &["multipart/form-data"];
const HTTP_METHODS: &[&str] = &[
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

fn main() {
    let spec_path = "openapi/immich-openapi-specs.json";
    println!("cargo:rerun-if-changed={spec_path}");

    let spec_file = std::fs::File::open(spec_path).expect("failed to open Immich OpenAPI spec");
    let mut spec: serde_json::Value =
        serde_json::from_reader(spec_file).expect("failed to parse Immich OpenAPI spec");

    for operation_id in remove_unsupported_operations(&mut spec) {
        println!("cargo:warning=skipping unsupported Immich operation `{operation_id}`");
    }

    let spec = serde_json::from_value(spec).expect("failed to parse patched Immich OpenAPI spec");
    let mut generator = progenitor::Generator::default();

    let tokens = generator
        .generate_tokens(&spec)
        .expect("failed to generate Immich SDK");
    let syntax_tree = syn::parse2(tokens).expect("failed to parse generated Immich SDK");
    let content = prettyplease::unparse(&syntax_tree);

    let out_path = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR is not set"))
        .join("codegen.rs");
    std::fs::write(out_path, content).expect("failed to write generated Immich SDK");
}

fn remove_unsupported_operations(spec: &mut serde_json::Value) -> Vec<String> {
    let mut skipped = Vec::new();
    let Some(paths) = spec
        .get_mut("paths")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return skipped;
    };

    for path in paths.values_mut() {
        let Some(path) = path.as_object_mut() else {
            continue;
        };

        let methods_to_remove = HTTP_METHODS
            .iter()
            .filter_map(|method| {
                let operation = path.get(*method)?;
                has_unsupported_request_content_type(operation).then(|| {
                    let operation_id = operation
                        .get("operationId")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or(*method)
                        .to_string();
                    ((*method).to_string(), operation_id)
                })
            })
            .collect::<Vec<_>>();

        for (method, operation_id) in methods_to_remove {
            path.remove(&method);
            skipped.push(operation_id);
        }
    }

    skipped
}

fn has_unsupported_request_content_type(operation: &serde_json::Value) -> bool {
    let Some(content) = operation
        .get("requestBody")
        .and_then(|request_body| request_body.get("content"))
        .and_then(serde_json::Value::as_object)
    else {
        return false;
    };

    UNSUPPORTED_REQUEST_CONTENT_TYPES
        .iter()
        .any(|content_type| content.contains_key(*content_type))
}
