/// Generate Python stub files (.pyi) from Rust PyO3 bindings.
///
/// Run via the wrapper (recommended; sets PATH and CARGO_MANIFEST_DIR):
///   pdm run stub_gen
///   or: python scripts/run_stub_gen.py
///
/// Or directly (requires CARGO_MANIFEST_DIR and, on Windows, Python DLL on PATH):
///   cargo run --bin stub_gen --features python,office
///
/// Output path is determined by [tool.maturin] in pyproject.toml (e.g.
/// python/pdf_oxide/pdf_oxide/__init__.pyi for module pdf_oxide.pdf_oxide).
/// This binary is invoked automatically in the release workflow before building
/// Python wheels (see .github/workflows/release.yml).
fn main() -> pyo3_stub_gen::Result<()> {
    // Enable logging so errors from stub generation are visible.
    env_logger::init();
    let stub = pdf_oxide::stub_info()?;
    stub.generate()?;
    Ok(())
}
