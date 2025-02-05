use std::{error::Error, fs, io::Write};
use substreams_ethereum::Abigen;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    compile_abi()?;
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
