use std::fs;
use std::path::Path;
use toml::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("Cargo.toml");
    let read = fs::read_to_string(path)
        .expect("Failed to read Cargo.toml");
    let toml: Value = read.parse::<Value>().expect("Failed to parse Cargo.toml");

    if let Some(raftify_dep) = toml.get("dependencies").and_then(|deps| deps.get("raftify")) {
        let version = raftify_dep.get("version").unwrap().as_str().unwrap();
        let features = raftify_dep.get("features").unwrap().as_array().unwrap().iter().map(|f| f.as_str().unwrap()).collect::<Vec<_>>().join(", ");
        println!("cargo:rustc-env=RAFTIFY_VERSION={}", &version[1..]);
        println!("cargo:rustc-env=RAFTIFY_FEATURES={}", features);
    }

    built::write_built_file().expect("Failed to acquire build-time information");
    Ok(())
}
