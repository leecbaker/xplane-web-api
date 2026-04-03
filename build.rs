use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let spec_path = "openapi/xplane-web-api-v3.yaml";
    println!("cargo:rerun-if-changed={spec_path}");

    let file = File::open(spec_path)
        .unwrap_or_else(|error| panic!("Failed to open OpenAPI file `{spec_path}`: {error}"));
    let spec: openapiv3::OpenAPI = yaml_serde::from_reader(file)
        .unwrap_or_else(|error| panic!("Failed to parse OpenAPI YAML `{spec_path}`: {error}"));

    let mut rest_generator = progenitor::Generator::default();
    let rest_tokens = rest_generator
        .generate_tokens(&spec)
        .unwrap_or_else(|error| panic!("Failed to generate client tokens from OpenAPI: {error}"));

    let rest_ast = syn::parse2(rest_tokens)
        .unwrap_or_else(|error| panic!("Failed to parse generated client tokens: {error}"));
    let rest_generated = prettyplease::unparse(&rest_ast);

    let out_dir =
        env::var("OUT_DIR").unwrap_or_else(|error| panic!("Missing OUT_DIR env var: {error}"));
    let rest_output_path = Path::new(&out_dir).join("xplane_web_api.rs");
    std::fs::write(&rest_output_path, rest_generated).unwrap_or_else(|error| {
        panic!(
            "Failed to write generated client source to `{}`: {error}",
            rest_output_path.display()
        )
    });
}
