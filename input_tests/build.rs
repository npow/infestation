use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("input_tests.rs");

    let scenarios_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    println!("cargo:rerun-if-changed={}", scenarios_dir.display());

    let mut input_files: HashMap<String, String> = HashMap::new();
    let mut output_files: HashMap<String, String> = HashMap::new();

    if let Ok(entries) = fs::read_dir(&scenarios_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            println!("cargo:rerun-if-changed={}", path.display());

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let ext = path.extension().and_then(|e| e.to_str());

            if ext == Some("json") {
                if let Some(base) = stem.strip_suffix("_i") {
                    input_files.insert(base.to_string(), path.display().to_string());
                } else if let Some(base) = stem.strip_suffix("_o") {
                    output_files.insert(base.to_string(), path.display().to_string());
                }
            }
        }
    }

    let mut code = String::new();
    let mut tests: Vec<(String, String)> = Vec::new();

    for (base_name, input_path) in &input_files {
        let output_path = output_files.get(base_name).cloned().unwrap_or_else(|| {
            let path = scenarios_dir.join(format!("{}_o.json", base_name));
            fs::write(&path, "[]\n").ok();
            path.display().to_string()
        });

        tests.push((
            base_name.clone(),
            format!(
                "#[test]\nfn {}() {{\n    run_input_test(include_str!(\"{}\"), include_str!(\"{}\"), \"{}\");\n}}\n",
                base_name, input_path, output_path, output_path
            ),
        ));
    }

    tests.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, test_code) in tests {
        code.push_str(&test_code);
    }

    if fs::read_to_string(&dest_path).map_or(true, |existing| existing != code) {
        fs::write(&dest_path, code).unwrap();
    }
}
