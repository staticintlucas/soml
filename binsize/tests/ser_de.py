from utils import crates
from . import common

from pathlib import Path
import tempfile
from textwrap import dedent

def write_manifest(path: Path, crate: crates.Crate):
    version = f'path = "{crate.path}"' if crate.path is not None else f'version = "{crate.version}"'

    content = dedent(f"""
        [package]
        name = "ser-de"
        edition = "2024"

        [dependencies]
        serde = {{ version = "1.0", features = ["derive"] }}
        {crate.package} = {{ {version} }}
    """)
    (path / "Cargo.toml").write_text(content)

def write_main(path: Path, crate: crates.Crate):
    package = crate.package.replace("-", "_")

    content = dedent(f"""\
        use serde::{{Serialize, Deserialize}};
        use {package}::{{from_str, to_string}};

        #[derive(Deserialize, Serialize)]
        #[serde(untagged)]
        enum Value {{
            String(String),
            Integer(i64),
            Float(f64),
            Boolean(bool),
            Array(Vec<Value>),
            Table(std::collections::HashMap<String, Value>),
        }}

        fn main() {{
            let input = std::fs::read_to_string("test1.toml").unwrap();
            let value: Value = from_str(&input).unwrap();
            let output = to_string(&value).unwrap();
            std::fs::write("test2.toml", output).unwrap();
        }}
    """)
    src = path / "src"
    src.mkdir(parents=True, exist_ok=True)
    (src / "main.rs").write_text(content)

def run_test(crate: crates.Crate) -> crates.Results:
    with tempfile.TemporaryDirectory() as tmpdir:
        path = Path(tmpdir)

        write_manifest(path, crate)
        write_main(path, crate)

        return common.run_test(path, crate)
