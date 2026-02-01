use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

const ACTIONS: &[(&str, &str)] = &[
    ("north", "Action::Move(Dir4::North)"),
    ("south", "Action::Move(Dir4::South)"),
    ("east", "Action::Move(Dir4::East)"),
    ("west", "Action::Move(Dir4::West)"),
    ("stall", "Action::Stall"),
];

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("scenario_tests.rs");

    let scenarios_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("scenarios");
    println!("cargo:rerun-if-changed={}", scenarios_dir.display());

    let mut before_files: HashMap<String, String> = HashMap::new();
    let mut after_files: Vec<(String, String, String)> = Vec::new();

    if let Ok(entries) = fs::read_dir(&scenarios_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            println!("cargo:rerun-if-changed={}", path.display());

            if path.extension().is_some_and(|e| e == "csv") {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                let mut found_action = false;
                for &(action_suffix, _) in ACTIONS {
                    if let Some(base) = stem.strip_suffix(&format!("_{}", action_suffix)) {
                        after_files.push((
                            base.to_string(),
                            action_suffix.to_string(),
                            path.display().to_string(),
                        ));
                        found_action = true;
                        break;
                    }
                }

                if !found_action {
                    before_files.insert(stem.to_string(), path.display().to_string());
                }
            }
        }
    }

    let mut code = String::new();
    let mut tests: Vec<(String, String)> = Vec::new();

    for (base_name, action, after_path) in &after_files {
        let before_path = before_files.get(base_name).unwrap_or_else(|| {
            panic!("No before file for {}_{}: expected {}.csv", base_name, action, base_name)
        });

        let action_expr = ACTIONS.iter().find(|&&(a, _)| a == action).unwrap().1;
        let test_name = format!("{}_{}", base_name, action);

        tests.push((
            test_name.clone(),
            format!(
                "scenario_test!({}, include_str!(\"{}\"), include_str!(\"{}\"), {}, \"{}\");\n",
                test_name, before_path, after_path, action_expr, after_path
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
