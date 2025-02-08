#![allow(clippy::unwrap_used, clippy::too_many_lines)]

use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
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
    let (mut valid_tests, mut invalid_tests) = (HashMap::new(), HashMap::new());
    for test in tests.lines() {
        let test = test.trim();
        if test.is_empty() || test.starts_with('#') {
            continue;
        }
        let path = PathBuf::from(test);
        if !path.extension().is_some_and(|ext| ext == "toml") {
            continue;
        }
        let mut path_iter = path.iter();
        let valid = path_iter.next().unwrap();
        let mod_path = {
            let mut temp = path_iter
                .map(OsStr::to_str)
                .collect::<Option<Vec<_>>>()
                .unwrap();
            temp.pop().unwrap();
            temp.join("_")
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_")
        };
        if valid == "valid" {
            valid_tests
                .entry(mod_path)
                .or_insert_with(Vec::new)
                .push(path);
        } else {
            invalid_tests
                .entry(mod_path)
                .or_insert_with(Vec::new)
                .push(path);
        }
    }

    // Output directory
    let out_dir = env::var_os("OUT_DIR").unwrap();

    // Output file for valid deserialization tests
    let out_file = Path::join(out_dir.as_ref(), "toml-test-de-valid.rs");
    println!("cargo:rustc-env=TOML_TEST_DE_VALID={}", out_file.display());
    let mut file = File::create(&out_file).unwrap();

    // Iterate over valid test files and create a deserialization test for each
    for (mod_path, tests) in &valid_tests {
        if !mod_path.is_empty() {
            writeln!(file, "mod {mod_path} {{").unwrap();
        }
        for test in tests {
            let test_name = test
                .with_extension("")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_");

            let toml_path = toml_test_dir.join(test);
            let json_path = toml_path.with_extension("json");

            writeln!(
                file,
                include_str!("tests/template-de-valid"),
                test_name = test_name,
                toml_path = toml_path.display(),
                json_path = json_path.display()
            )
            .unwrap();
        }
        if !mod_path.is_empty() {
            writeln!(file, "}}").unwrap();
        }
    }

    // Output file for invalid deserialization tests
    let out_file = Path::join(out_dir.as_ref(), "toml-test-de-invalid.rs");
    println!(
        "cargo:rustc-env=TOML_TEST_DE_INVALID={}",
        out_file.display()
    );
    let mut file = File::create(&out_file).unwrap();

    // Iterate over invalid test files and create a deserialization test for each
    for (mod_path, tests) in invalid_tests {
        if !mod_path.is_empty() {
            writeln!(file, "mod {mod_path} {{").unwrap();
        }
        for test in tests {
            let test_name = test
                .with_extension("")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_");

            let toml_path = toml_test_dir.join(test);

            writeln!(
                file,
                include_str!("tests/template-de-invalid"),
                test_name = test_name,
                toml_path = toml_path.display()
            )
            .unwrap();
        }
        if !mod_path.is_empty() {
            writeln!(file, "}}").unwrap();
        }
    }

    // Output file for serialization tests
    let out_file = Path::join(out_dir.as_ref(), "toml-test-ser.rs");
    println!("cargo:rustc-env=TOML_TEST_SER={}", out_file.display());
    let mut file = File::create(&out_file).unwrap();

    // Iterate over invalid test files and create a deserialization test for each
    for (mod_path, tests) in valid_tests {
        if !mod_path.is_empty() {
            writeln!(file, "mod {mod_path} {{").unwrap();
        }
        for test in tests {
            let test_name = test
                .with_extension("")
                .file_name()
                .unwrap()
                .to_string_lossy()
                .replace(|ch| !char::is_ascii_alphanumeric(&ch), "_");

            let toml_path = toml_test_dir.join(test);
            let json_path = toml_path.with_extension("json");

            writeln!(
                file,
                include_str!("tests/template-ser"),
                test_name = test_name,
                json_path = json_path.display()
            )
            .unwrap();
        }
        if !mod_path.is_empty() {
            writeln!(file, "}}").unwrap();
        }
    }
}
