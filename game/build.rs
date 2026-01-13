use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;

const FONT_PATH: &str = "../assets/DejaVuSans.ttf";
const FONT_URL: &str = "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-fonts-ttf-2.37.tar.bz2";

const MINIQUAD_JS_FILES: &[(&str, &str)] = &[
    (
        "../js/gl.js",
        "https://raw.githubusercontent.com/not-fl3/miniquad/master/js/gl.js",
    ),
    (
        "../js/sapp_jsutils.js",
        "https://raw.githubusercontent.com/not-fl3/sapp-jsutils/master/js/sapp_jsutils.js",
    ),
];

fn main() {
    download_font();
    download_miniquad_js();
    embed_levels();
}

fn download_font() {
    if Path::new(FONT_PATH).exists() {
        return;
    }

    eprintln!("Downloading DejaVu Sans font...");

    let response = ureq::get(FONT_URL)
        .call()
        .expect("Failed to download font archive");

    let reader = bzip2::read::BzDecoder::new(response.into_body().into_reader());
    let mut archive = tar::Archive::new(reader);

    for entry in archive.entries().expect("Failed to read archive") {
        let mut entry = entry.expect("Failed to read archive entry");
        let path = entry.path().expect("Failed to get entry path");

        if path.ends_with("DejaVuSans.ttf") {
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .expect("Failed to read font data");

            fs::create_dir_all(Path::new(FONT_PATH).parent().unwrap()).unwrap();
            fs::write(FONT_PATH, &data).expect("Failed to write font file");

            eprintln!("Downloaded DejaVuSans.ttf");
            return;
        }
    }

    panic!("DejaVuSans.ttf not found in archive");
}

fn download_miniquad_js() {
    for &(path, url) in MINIQUAD_JS_FILES {
        if Path::new(path).exists() {
            continue;
        }

        eprintln!("Downloading {path}...");

        let response = ureq::get(url)
            .call()
            .unwrap_or_else(|e| panic!("Failed to download {url}: {e}"));

        let mut data = Vec::new();
        response
            .into_body()
            .into_reader()
            .read_to_end(&mut data)
            .unwrap_or_else(|e| panic!("Failed to read {url}: {e}"));

        fs::create_dir_all(Path::new(path).parent().unwrap()).unwrap();
        fs::write(path, &data).unwrap_or_else(|e| panic!("Failed to write {path}: {e}"));

        eprintln!("Downloaded {path}");
    }
}

fn collect_levels(dir: &Path, prefix: &str, levels: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let subdir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let new_prefix = if prefix.is_empty() {
                subdir_name.to_string()
            } else {
                format!("{}/{}", prefix, subdir_name)
            };
            println!("cargo:rerun-if-changed={}", path.display());
            collect_levels(&path, &new_prefix, levels);
        } else if path.extension().is_some_and(|e| e == "csv")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
        {
            let level_name = if prefix.is_empty() {
                stem.to_string()
            } else {
                format!("{}/{}", prefix, stem)
            };
            levels.push(level_name);
            println!("cargo:rerun-if-changed={}", path.display());
            let json_path = path.with_extension("json");
            if json_path.exists() {
                println!("cargo:rerun-if-changed={}", json_path.display());
            }
        }
    }
}

fn embed_levels() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("levels.rs");

    let levels_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../levels");

    println!("cargo:rerun-if-changed={}", levels_dir.display());

    let mut levels: Vec<String> = Vec::new();
    collect_levels(&levels_dir, "", &mut levels);

    levels.sort();

    let mut code = String::new();
    code.push_str("pub(crate) static LEVEL_DATA: &[(&str, &str, &str)] = &[\n");

    for name in &levels {
        let rel_csv = format!("../levels/{}.csv", name);
        let rel_json = format!("../levels/{}.json", name);

        code.push_str(&format!(
            "    ({:?}, include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{rel_csv}\")), include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{rel_json}\"))),\n",
            name
        ));
    }

    code.push_str("];\n");

    // Only write if content changed to avoid unnecessary recompilation
    let should_write = match fs::read_to_string(&dest_path) {
        Ok(existing) => existing != code,
        Err(_) => true,
    };
    if should_write {
        fs::write(&dest_path, code).unwrap();
    }
}
