"""PEP 723 inline script metadata (the blocks `uv run` reads): the PyPI
packages a script or cell names, installed with uv into a folder kept by
their set (built for this Python) and put on sys.path."""

from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
import sys

BLOCK = re.compile(r"(?m)^# /// (?P<type>[a-zA-Z0-9-]+)$\s(?P<content>(^#(| .*)$\s)+)^# ///$")


def metadata(text: str):
    """The `script` block's TOML as a dict, or None."""
    for m in BLOCK.finditer(text if text.endswith("\n") else text + "\n"):
        if m.group("type") != "script":
            continue
        content = "".join(line[2:] if line.startswith("# ") else line[1:]
                          for line in m.group("content").splitlines(keepends=True))
        try:
            import tomllib
        except ImportError:  # (Python < 3.11)
            return None
        return tomllib.loads(content)
    return None


def cache_dir():
    base = os.environ.get("XDG_CACHE_HOME") or os.path.join(os.path.expanduser("~"), ".cache")
    return os.path.join(base, "tessh", "deps")


def uv_command():
    uv = shutil.which("uv")
    if uv:
        return [uv]
    if shutil.which("nix"):
        return ["nix", "run", "--quiet", "nixpkgs#uv", "--"]
    return None


def ensure(dependencies, requires_python=None, quiet=False):
    """Install `dependencies` (once) and put them on sys.path."""
    if requires_python:
        try:
            from packaging.specifiers import SpecifierSet
            here = ".".join(map(str, sys.version_info[:3]))
            if here not in SpecifierSet(requires_python):
                print(f"tessh: this script wants Python {requires_python}; this is {here}", file=sys.stderr)
        except Exception:
            pass
    deps = sorted(set(dependencies))
    if not deps:
        return None
    key = hashlib.sha256(("\n".join(deps) + "\n" + sys.version).encode()).hexdigest()[:16]
    target = os.path.join(cache_dir(), key)
    done = os.path.join(target, ".tessh-done")
    if not os.path.exists(done):
        uv = uv_command()
        if uv is None:
            raise RuntimeError("this script needs packages (" + ", ".join(deps) + ") and uv isn't installed (nor Nix to get it)")
        if not quiet:
            print("tessh: installing " + ", ".join(deps) + " with uv…", file=sys.stderr)
        os.makedirs(target, exist_ok=True)
        r = subprocess.run(uv + ["pip", "install", "--quiet", "--python", sys.executable, "--target", target] + deps)
        if r.returncode != 0:
            raise RuntimeError("uv couldn't install " + ", ".join(deps))
        open(done, "w").close()
    if target not in sys.path:
        sys.path.insert(0, target)
    return target


def ensure_from_source(text: str):
    if "# /// script" not in text:
        return None
    meta = metadata(text)
    if not meta:
        return None
    return ensure(meta.get("dependencies") or [], meta.get("requires-python"))
