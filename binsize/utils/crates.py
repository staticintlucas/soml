import tomllib
from pathlib import Path
from typing import NamedTuple, Any

from .paths import soml_root

class Results(NamedTuple):
    version: str | None
    size: int

class Crate(NamedTuple):
    package: str
    notes: str
    version: str | None
    path: Path | None
    default_features: bool
    features: list[str]
    maintained: bool
    toml_ver: str
    url: str
    footnotes: list[str]

def load(path: Path | str) -> list[Crate]:
    return loads(Path(path).read_text())

def loads(crates_toml: str) -> list[Crate]:
    crates_toml: dict[str, dict[str, Any]] = tomllib.loads(crates_toml)
    crates = []

    for _key, props in crates_toml.items():
        assert "version" in props or "path" in props

        if "path" in props:
            if not Path(props["path"]).is_absolute():
                props["path"] = (soml_root() / props["path"]).resolve()
            else:
                props["path"] = Path(props["path"]).resolve()

        crates.append(Crate(
            package=str(props["package"]),
            notes=str(props["notes"]) if "notes" in props else None,
            version=str(props["version"]) if "version" in props else None,
            path=props["path"] if "path" in props else None,
            default_features=bool(props.get("default-features", True)),
            features=list(str(f) for f in props.get("features", [])),
            maintained=bool(props["maintained"]),
            toml_ver=str(props["toml-ver"]),
            url=str(props["url"]),
            footnotes=list(str(f) for f in props.get("footnotes", [])),
        ))

    return crates
