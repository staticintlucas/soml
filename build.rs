#![allow(clippy::unwrap_used)]

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    // Allow tests to be created using a different test index
    let test_index = env::var("TOML_TEST_INDEX")
        .unwrap_or_else(|_| "tests/toml-test/tests/files-toml-1.0.0".into());
    println!("cargo:rerun-if-env-changed=TOML_TEST_INDEX");

    // Where toml-test files are located
    let toml_test_dir = PathBuf::from("tests/toml-test/tests");
    println!("cargo:rerun-if-changed={}", toml_test_dir.display());

    // Read the test index and split files into valid/invalid
    let tests = fs::read_to_string(test_index).unwrap();
    let (valid_tests, invalid_tests): (Vec<_>, Vec<_>) = tests
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(PathBuf::from)
        .filter(|file| file.extension().is_some_and(|ext| ext == "toml"))
        .partition(|file| file.starts_with("valid"));

    // Create the output file
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let out_file = Path::join(out_dir.as_ref(), "toml-test-tests.rs");
    println!("cargo:rustc-env=TOML_TEST_TESTS={}", out_file.display());
    let mut file = File::create(&out_file).unwrap();

    // Iterate over valid test files and create a test for each
    for test in valid_tests {
        let test_name = format!(
            "toml_test_{}",
            test.with_extension("")
                .to_string_lossy()
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_")
        );

        let toml_path = toml_test_dir.join(test);
        let json_path = toml_path.with_extension("json");

        writeln!(
            file,
            include_str!("tests/template-valid"),
            test_name = test_name,
            toml_path = toml_path.display(),
            json_path = json_path.display()
        )
        .unwrap();
    }

    for test in invalid_tests {
        let test_name = format!(
            "toml_test_{}",
            test.with_extension("")
                .to_string_lossy()
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_")
        );

        let toml_path = toml_test_dir.join(test);
        writeln!(
            file,
            include_str!("tests/template-invalid"),
            test_name = test_name,
            toml_path = toml_path.display()
        )
        .unwrap();
    }
}
