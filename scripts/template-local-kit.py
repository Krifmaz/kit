#!/usr/bin/env python3
"""Point a generated app's kit dependencies at a local kit checkout.

    python3 scripts/template-local-kit.py <app dir> <kit dir>

Every `kit-x = { git = "https://github.com/Krifmaz/kit", tag = "…"… }` line in the app's Cargo.toml files becomes
`kit-x = { path = "<kit dir>/crates/kit-x"… }` (relative to that Cargo.toml, so the pair can be copied together, as
the kit's CI does for the Docker build), so the kit's CI builds the generated app against the pull
request's own crates (a [patch] would still need the tag to exist). Other attributes on the line are kept.
"""
import os
import pathlib
import re
import sys

app, kit = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]).resolve()
line = re.compile(r'^(kit-[a-z-]+)(\s*=\s*\{\s*)git = "https://github\.com/Krifmaz/kit",\s*tag = "[^"]*"', re.M)
changed = 0
for toml in app.rglob("Cargo.toml"):
    text = toml.read_text()
    rel = os.path.relpath(kit / "crates", toml.parent.resolve())
    new, n = line.subn(lambda m: f'{m.group(1)}{m.group(2)}path = "{rel}/{m.group(1)}"', text)
    if n:
        toml.write_text(new)
        changed += n
        print(f"{toml}: {n} kit dependencies now local")
if not changed:
    sys.exit("no kit dependency found to point at the local kit")
