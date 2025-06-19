import functools
from pathlib import Path
import sys
import tomllib

@functools.cache
def script() -> Path:
    script = Path(sys.argv[0]).resolve()
    assert str(script).startswith(str(soml_root()))
    return script.relative_to(soml_root())

@functools.cache
def script_root() -> Path:
    root = Path(sys.argv[0]).parent.resolve()
    assert str(Path(__file__).resolve()).startswith(str(root))
    return root

@functools.cache
def soml_root() -> Path:
    root = script_root()
    while not (root.is_dir() and (root / "Cargo.toml").exists()):
        root = root.parent

    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    assert manifest["package"]["name"] == "soml"

    return root

def repo_url(path: Path | str = "") -> str:
    manifest = tomllib.loads((soml_root() / "Cargo.toml").read_text())
    return f"{manifest["package"]["repository"]}/blob/main/{path}"
