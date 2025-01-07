use prost_build;
use std::{error::Error, fs, io::Write};
use substreams_ethereum::Abigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    compile_abi()?;
    compile_proto()?;
    Ok(())
}

fn compile_proto() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = &["proto/sf/bstream/v1/bstream.proto"];
    let proto_include_dirs = &["proto"];
    let out_dir = "src/proto";
    std::fs::create_dir_all(out_dir)?;

    prost_build::Config::new()
        .compile_well_known_types()
        .out_dir(out_dir)
        .compile_protos(proto_files, proto_include_dirs)?;

    // Tell Cargo to rerun the build script if the proto file changes
    for proto_file in proto_files {
        println!("cargo:rerun-if-changed={}", proto_file);
    }
    Ok(())
}

fn compile_abi() -> Result<(), Box<dyn Error>> {
    let abi_folder = "abi";
    let output_folder = "src/abi";
    let mut mod_rs_content = "#![allow(clippy::all)]\n".to_string();

    let generate_json_paths = |entry: fs::DirEntry| {
        entry
            .file_name()
            .to_string_lossy()
            .split_once('.')
            .filter(|(_, ext)| *ext == "json")
            .map(|(contract_name, ext)| {
                (
                    contract_name.to_string(),
                    format!("{}/{}.{}", abi_folder, contract_name, ext),
                    format!("{}/{}.rs", output_folder, contract_name),
                )
            })
    };

    let abi_codegen = |(contract_name, input_path, output_path)| -> Result<(), Box<dyn Error>> {
        mod_rs_content.push_str(&format!("pub mod {};\n", contract_name));
        Abigen::new(&contract_name, &input_path)?
            .generate()?
            .write_to_file(&output_path)
            .map_err(|e| e.into())
    };

    fs::read_dir(abi_folder)?
        .filter_map(Result::ok)
        .filter_map(generate_json_paths)
        .map(abi_codegen)
        .collect::<Result<(), Box<dyn Error>>>()?;

    fs::File::create(format!("{}/mod.rs", output_folder))?.write_all(mod_rs_content.as_bytes())?;

    Ok(())
}
