fn main() {
    tauri_build::build();

    let content = std::fs::read_to_string("../package.json")
        .expect("package.json not found — build script depends on it");
    let version = content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("\"version\"") {
                trimmed
                    .split(':')
                    .nth(1)?
                    .trim()
                    .trim_matches(|c| c == ',' || c == '"' || c == ' ')
                    .to_string()
                    .into()
            } else {
                None
            }
        })
        .expect("version field not found in package.json");
    println!("cargo:rustc-env=APP_VERSION={}", version);
    println!("cargo:rerun-if-changed=../package.json");
}
