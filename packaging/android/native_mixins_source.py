"""Module files the game names through a parent folder, moved next to the module that names them, in a build's copy.

    native_parent_paths.py SOURCE

horizon-mixins refuses a `#[path = "../x.rs"]` (E_INVALID_PATH: no '.' or '..'), and since the fork of 2026-11-14
the game has some outside tests: `game/mobs/creatures/mod.rs` names five files of `game/creatures/` that way. In
SOURCE (a copy that native_build.sh and tools/starling/hnmod_compat.sh prepare, never the checkout), each such
file moves into the folder its `#[path]` is read from, and the attribute names it there. The module's Rust path
does not change, so mods target it as before. Refused, leaving SOURCE as it was: a file named twice, a moved file
that names modules of its own (they would be looked for in the new folder), or a name already taken there.
Modules under `#[cfg(test)]` are left alone: horizon-mixins skips them.
"""

import pathlib
import re
import sys

ATTR = re.compile(r'^(\s*)#\[path\s*=\s*"([^"]*\.\.[^"]*)"\]\s*$')
MOD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+\w+\s*;")
CHILD = re.compile(r"^\s*(pub(\([^)]*\))?\s+)?mod\s+\w+\s*;", re.M)


def main() -> None:
    source = pathlib.Path(sys.argv[1]).resolve()
    src = source / "crates/horizon-client/src"
    moves = {}
    edits = []
    for file in sorted(src.rglob("*.rs")):
        lines = file.read_text(encoding="utf-8").split("\n")
        for i, line in enumerate(lines):
            m = ATTR.match(line)
            if not m:
                continue
            # The item it is on: the next line that is not an attribute or a comment.
            j = i + 1
            attrs = [line]
            while j < len(lines) and lines[j].strip().startswith(("#[", "//")):
                attrs.append(lines[j])
                j += 1
            k = i - 1
            while k >= 0 and lines[k].strip().startswith(("#[", "///")):
                attrs.append(lines[k])
                k -= 1
            if j >= len(lines) or not MOD.match(lines[j]) or any("cfg(test)" in a for a in attrs):
                continue
            target = (file.parent / m.group(2)).resolve()
            # A test file's own fixtures outside the game (horizon-mod-wasm's, the book's): the test module above
            # them is under #[cfg(test)], which horizon-mixins does not read.
            if not target.is_file() or src not in target.parents:
                continue
            if target in moves:
                sys.exit(f"native_parent_paths: {target.relative_to(source)} is named twice")
            if CHILD.search(target.read_text(encoding="utf-8")):
                sys.exit(f"native_parent_paths: {target.relative_to(source)} names modules of its own")
            new = file.parent / target.name
            if new.exists():
                sys.exit(f"native_parent_paths: {new.relative_to(source)} is taken")
            moves[target] = new
            edits.append((file, i, f'{m.group(1)}#[path = "{target.name}"]'))
    for target, new in moves.items():
        target.rename(new)
    by_file = {}
    for file, i, text in edits:
        by_file.setdefault(file, []).append((i, text))
    for file, changes in by_file.items():
        lines = file.read_text(encoding="utf-8").split("\n")
        for i, text in changes:
            lines[i] = text
        file.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    for target, new in moves.items():
        print(f"native_parent_paths: {target.relative_to(source)} -> {new.relative_to(source)}")


if __name__ == "__main__":
    main()
