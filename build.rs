fn main() {
    println!("cargo:rerun-if-changed=schemas/mensatt.graphql");

    cynic_codegen::register_schema("mensatt")
        .from_sdl_file("schemas/mensatt.graphql")
        .expect("Failed to build schema from file")
        .as_default()
        .expect("Could not set schema as default");
}
