use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("scenario_tests.rs");

    let scenarios_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    println!("cargo:rerun-if-changed={}", scenarios_dir.display());

    // Collect scenario triplets: _1.csv, _2.csv, .json
    let mut before_files: HashMap<String, String> = HashMap::new();
    let mut after_files: HashMap<String, String> = HashMap::new();
    let mut json_files: HashMap<String, String> = HashMap::new();

    if let Ok(entries) = fs::read_dir(&scenarios_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            println!("cargo:rerun-if-changed={}", path.display());

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let ext = path.extension().and_then(|e| e.to_str());

            match ext {
                Some("csv") => {
                    if let Some(base) = stem.strip_suffix("_1") {
                        before_files.insert(base.to_string(), path.display().to_string());
                    } else if let Some(base) = stem.strip_suffix("_2") {
                        after_files.insert(base.to_string(), path.display().to_string());
                    }
                }
                Some("json") => {
                    if let Some(base) = stem.strip_suffix("_i") {
                        json_files.insert(base.to_string(), path.display().to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let mut code = String::new();
    let mut tests: Vec<(String, String)> = Vec::new();

    for (base_name, json_path) in &json_files {
        let before_path = before_files.get(base_name).unwrap_or_else(|| {
            panic!("No before file for {}: expected {}_1.csv", base_name, base_name)
        });

        // If _2.csv doesn't exist yet, create an empty placeholder so UPDATE_SNAPSHOTS can generate it
        let after_path = after_files.get(base_name).cloned().unwrap_or_else(|| {
            let path = scenarios_dir.join(format!("{}_2.csv", base_name));
            fs::write(&path, "").ok();
            path.display().to_string()
        });

        tests.push((
            base_name.clone(),
            format!(
                "#[test]\nfn {}() {{\n    run_scenario_test(include_str!(\"{}\"), include_str!(\"{}\"), \"{}\", \"{}\");\n}}\n",
                base_name, before_path, &after_path, &after_path, json_path
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
