#!/bin/bash

set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
from pathlib import Path
import os
import re
import subprocess
import sys

tracked = subprocess.check_output(
    ["git", "ls-files", "-z", "*.rs"]
).decode().split("\0")
paths = [Path(path) for path in tracked if path]


def source_root(path: Path) -> Path | None:
    parts = path.parts
    if parts[:2] == ("app", "src"):
        return Path("app/src")
    if len(parts) >= 4 and parts[0] == "crates" and parts[2] == "src":
        return Path(*parts[:3])
    return None

source_paths = [path for path in paths if source_root(path) is not None]
contents = {
    path: path.read_text(encoding="utf-8", errors="replace")
    for path in source_paths
}
path_attribute = re.compile(r'\bpath\s*=\s*"([^"]+)"')


declared_path_targets = {
    Path(os.path.normpath(owner.parent / match.group(1)))
    for owner, text in contents.items()
    for match in path_attribute.finditer(text)
}


def is_automatic_root(path: Path, root: Path) -> bool:
    relative = path.relative_to(root)
    if path.name == "mod.rs":
        return True
    if relative.parts in (("lib.rs",), ("main.rs",)):
        return True
    if len(relative.parts) == 2 and relative.parts[0] == "bin":
        return path.suffix == ".rs"
    return len(relative.parts) >= 2 and relative.parts[0] == "bin" and path.name == "main.rs"


def is_declared(path: Path, root: Path) -> bool:
    if path in declared_path_targets:
        return True
    stem = re.escape(path.stem)
    module_declaration = re.compile(
        rf'^\s*(?:pub(?:\s*\([^)]*\))?\s+)?(?:unsafe\s+)?mod\s+(?:r#)?{stem}\s*;',
        re.MULTILINE,
    )

    if path.parent == root:
        candidates = [root / "lib.rs", root / "main.rs"]
    else:
        candidates = [path.parent / "mod.rs", Path(f"{path.parent}.rs")]
    candidates.extend(
        owner for owner in declared_path_targets if owner.parent == path.parent
    )

    for candidate in dict.fromkeys(candidates):
        text = contents.get(candidate)
        if text is None:
            continue
        if module_declaration.search(text):
            return True
        for match in path_attribute.finditer(text):
            target = Path(os.path.normpath(candidate.parent / match.group(1)))
            if target == path:
                return True
    return False


orphans = []
for path in source_paths:
    root = source_root(path)
    assert root is not None
    if is_automatic_root(path, root):
        continue
    if not is_declared(path, root):
        orphans.append(path)

if not orphans:
    print("No orphaned Rust modules found.")
    sys.exit(0)

for orphan in sorted(orphans):
    print(f"ORPHAN: {orphan}")
sys.exit(1)
PY
