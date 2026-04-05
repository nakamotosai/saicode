use std::fs;
use std::path::PathBuf;

fn extract_version(package_json: &str) -> Option<String> {
  let key_index = package_json.find("\"version\"")?;
  let after_key = &package_json[key_index + "\"version\"".len()..];
  let colon_index = after_key.find(':')?;
  let after_colon = after_key[colon_index + 1..].trim_start();
  let rest = after_colon.strip_prefix('"')?;
  let end_quote = rest.find('"')?;
  Some(rest[..end_quote].to_string())
}

fn main() {
  let manifest_dir = PathBuf::from(
    std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"),
  );
  let package_json_path = manifest_dir
    .parent()
    .and_then(|path| path.parent())
    .expect("native launcher repo layout changed")
    .join("package.json");

  println!("cargo:rerun-if-changed={}", package_json_path.display());

  if let Ok(package_json) = fs::read_to_string(&package_json_path) {
    if let Some(version) = extract_version(&package_json) {
      println!("cargo:rustc-env=SAICODE_PACKAGE_VERSION={version}");
    }
  }
}
