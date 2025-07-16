from utils import crates
from . import common

from pathlib import Path
import tempfile
from textwrap import dedent

def write_manifest(path: Path):
    content = dedent(f"""
        [package]
        name = "reference"
        edition = "2024"
    """)
    (path / "Cargo.toml").write_text(content)

def write_main(path: Path):
    content = dedent(f"""\
        fn main() {{
            let input = std::fs::read_to_string("test1.toml").unwrap();
            let output = input;
            std::fs::write("test2.toml", output).unwrap();
        }}
    """)
    src = path / "src"
    src.mkdir(parents=True, exist_ok=True)
    (src / "main.rs").write_text(content)


def run_test() -> crates.Results:
    with tempfile.TemporaryDirectory() as tmpdir:
        path = Path(tmpdir)

        write_manifest(path)
        write_main(path)

        return common.run_test(path, None)
