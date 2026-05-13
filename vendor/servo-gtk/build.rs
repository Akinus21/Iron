fn main() {
    prost_build::Config::new()
        .file_descriptor_set_path("proto_descriptor.bin")
        .out_dir("src")
        .compile_protos(&["proto/ipc.proto"], &["proto"])
        .unwrap();
}