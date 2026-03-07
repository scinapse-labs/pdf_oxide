#!/usr/bin/env python3
"""Run stub_gen binary with PATH set so the Python DLL is found on Windows.

The Rust binary (stub_gen.exe) links against the Python shared library; on
Windows it must find e.g. python3xx.dll. This script prepends all likely
Python DLL locations to PATH, then runs cargo so the child stub_gen.exe
can load the DLL.
"""
from __future__ import annotations

import os
import subprocess
import sys


def _dll_paths() -> list[str]:
    """Paths that might contain the Python DLL (python3xx.dll), in search order."""
    seen: set[str] = set()
    out: list[str] = []
    base_exe = getattr(sys, "base_executable", sys.executable)
    for p in (
        os.path.dirname(os.path.abspath(sys.executable)),
        os.path.dirname(os.path.abspath(base_exe)),
        os.path.abspath(sys.base_prefix),
        os.path.join(sys.base_prefix, "Scripts"),
        os.path.join(sys.base_prefix, "Library", "bin"),
    ):
        if p and os.path.isdir(p) and p not in seen:
            seen.add(p)
            out.append(p)
    return out


def main() -> int:
    # Project root (directory with Cargo.toml / pyproject.toml). stub_gen.exe expects CARGO_MANIFEST_DIR.
    project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(project_root)

    extra = os.pathsep.join(_dll_paths())
    env = os.environ.copy()
    env["PATH"] = extra + os.pathsep + env.get("PATH", "")
    env["CARGO_MANIFEST_DIR"] = project_root

    # Build first (cargo run may not pass env to the exe on Windows).
    # On failure, do not run stub_gen; return the build exit code.
    r = subprocess.run(
        ["cargo", "build", "--bin", "stub_gen", "--features", "python,office"],
        env=env,
    )
    if r.returncode != 0:
        return r.returncode

    # Run the exe directly so it definitely gets our PATH and CARGO_MANIFEST_DIR.
    exe_name = "stub_gen.exe" if sys.platform == "win32" else "stub_gen"
    exe = os.path.join("target", "debug", exe_name)
    if not os.path.isfile(exe):
        exe = os.path.join("target", "release", exe_name)
    return subprocess.run([exe], env=env, cwd=project_root).returncode


if __name__ == "__main__":
    sys.exit(main())
