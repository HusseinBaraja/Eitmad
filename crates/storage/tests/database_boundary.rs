use std::{fs, path::Path};

const SOURCE_EXTENSIONS: &[&str] = &["cs", "csproj", "swift", "rs", "toml"];
const FORBIDDEN: &[&str] = &[
    "eitmad.sqlite3",
    "microsoft.data.sqlite",
    "system.data.sqlite",
    "sqliteconnection",
    "rusqlite",
];

#[test]
fn shells_and_platform_adapters_cannot_open_product_storage() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("storage crate is inside the repository");
    for relative in ["shells", "platform-adapters"] {
        inspect_directory(&repository.join(relative));
    }
}

fn inspect_directory(directory: &Path) {
    for entry in fs::read_dir(directory).expect("boundary directory is readable") {
        let entry = entry.expect("boundary entry is readable");
        let path = entry.path();
        if path.is_dir() {
            inspect_directory(&path);
            continue;
        }
        let extension = path.extension().and_then(|value| value.to_str());
        if !extension.is_some_and(|value| SOURCE_EXTENSIONS.contains(&value)) {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("source file is UTF-8");
        let lower = contents.to_ascii_lowercase();
        for forbidden in FORBIDDEN {
            assert!(
                !lower.contains(forbidden),
                "{} contains forbidden database access marker {forbidden}",
                path.display()
            );
        }
    }
}
