from utils import crates

import json
from pathlib import Path
import subprocess

def run_test(path: Path, crate: crates.Crate | None) -> crates.Results:
    bloat_cmd = ["cargo", "bloat", "--release", "--message-format=json"]

    bloat = subprocess.run(bloat_cmd, cwd=path, capture_output=True)

    if bloat.returncode != 0:
        print(bloat.stderr.decode())
        raise subprocess.CalledProcessError(bloat.returncode, bloat.args)

    size = json.loads(bloat.stdout)["text-section-size"]

    if crate is not None:
        meta_cmd = ["cargo", "metadata", "--format-version=1"]
        meta = subprocess.run(meta_cmd, cwd=path, capture_output=True)
        if meta.returncode != 0:
            print(meta.stderr.decode())
            raise subprocess.CalledProcessError(meta.returncode, meta.args)

        [dep] = filter(lambda dep: dep["name"] == crate.package, json.loads(meta.stdout)["packages"])
        version = str(dep["version"])
    else:
        version = None

    return crates.Results(
        version=version,
        size=int(size),
    )
