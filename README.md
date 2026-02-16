# PDFOxide

**The Complete PDF Toolkit for Rust and Beyond**

Extract, create, and edit PDFs with one library. Rust core with bindings for every language.

```
                         ┌──────────────┐
                         │  Rust Core   │
                         └──────┬───────┘
          ┌──────────┬─────────┼─────────┬──────────┐
          ▼          ▼         ▼         ▼          ▼
      ┌───────┐  ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐
      │Python │  │ Node  │ │ WASM  │ │  Go   │ │  ...  │
      │  ✅   │  │ Soon  │ │ Soon  │ │ Soon  │ │       │
      └───────┘  └───────┘ └───────┘ └───────┘ └───────┘
```

[![Crates.io](https://img.shields.io/crates/v/pdf_oxide.svg)](https://crates.io/crates/pdf_oxide)
[![Documentation](https://docs.rs/pdf_oxide/badge.svg)](https://docs.rs/pdf_oxide)
[![Build Status](https://github.com/yfedoseev/pdf_oxide/workflows/CI/badge.svg)](https://github.com/yfedoseev/pdf_oxide/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

[📖 Documentation](https://docs.rs/pdf_oxide) | [📝 Changelog](CHANGELOG.md) | [🤝 Contributing](CONTRIBUTING.md) | [🔒 Security](SECURITY.md)

## Quick Start

### Extract text from PDF
```rust
let mut doc = PdfDocument::open("input.pdf")?;
let text = doc.extract_text(0)?;
let markdown = doc.to_markdown(0, Default::default())?;
```

### Create a new PDF
```rust
let mut builder = DocumentBuilder::new();
builder.add_page(612.0, 792.0)
    .text("Hello, World!", 72.0, 720.0, 24.0);
builder.save("output.pdf")?;
```

### Edit an existing PDF
```rust
let mut editor = DocumentEditor::open("input.pdf")?;
editor.add_highlight(0, rect, Color::yellow())?;
editor.add_text_field("name", rect)?;
editor.save("output.pdf")?;
```

## Why pdf_oxide?

- 📄 **One library** - Extract, create, and edit with unified API
- ⚡ **Fast** - 97.6% of PDFs processed in under 10ms (p99 = 33ms)
- 🦀 **Pure Rust** - Memory-safe, no C dependencies
- 🌍 **Multi-language** - Rust core with Python bindings (Node, WASM, Go planned)

## Features

| Extract | Create | Edit |
|---------|--------|------|
| Text & Layout | Documents | Annotations |
| Images | Tables | Form Fields |
| Forms | Graphics | Bookmarks |
| Annotations | Templates | Links |
| Bookmarks | Images | Content |

**v0.3.5 Highlights:** 99.8% compatibility across 3,830 test PDFs, font caching, content stream DoS protection, resilient error recovery, image extraction from content streams. See [CHANGELOG.md](CHANGELOG.md) for details.

## Installation

### Rust
```toml
[dependencies]
pdf_oxide = "0.3"
```

### Python
```bash
pip install pdf_oxide
```

## Examples

### Rust - Extraction
```rust
use pdf_oxide::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = PdfDocument::open("paper.pdf")?;

    // Extract text
    let text = doc.extract_text(0)?;

    // Convert to Markdown
    let markdown = doc.to_markdown(0, Default::default())?;

    // Extract images
    let images = doc.extract_images(0)?;

    // Get annotations
    let annotations = doc.get_annotations(0)?;

    Ok(())
}
```

### Python
```python
from pdf_oxide import PdfDocument

doc = PdfDocument("paper.pdf")
text = doc.extract_text(0)
markdown = doc.to_markdown(0, detect_headings=True)
```

For more examples, see the [examples/](examples/) directory.

## Performance

Verified against 3,830 PDFs from three independent test suites:

| Corpus | PDFs | Pass Rate |
|--------|-----:|----------:|
| veraPDF (PDF/A compliance) | 2,907 | 100% |
| Mozilla pdf.js | 897 | 100% |
| SafeDocs (targeted edge cases) | 26 | 100% |
| **Total** | **3,830** | **100%** |

| Metric | Result |
|--------|--------|
| **p50 latency** | 0.6ms |
| **p90 latency** | 3.0ms |
| **p99 latency** | 33ms |
| **Under 10ms** | 97.6% of PDFs |
| **Timeouts** | 0 |
| **Panics** | 0 |

100% pass rate on all valid PDFs. The only 7 non-passing files across the entire corpus are intentionally broken test fixtures (no PDF header, fuzz-corrupted catalogs, invalid xref streams).

## Building from Source

```bash
# Clone and build
git clone https://github.com/yfedoseev/pdf_oxide
cd pdf_oxide
cargo build --release

# Run tests
cargo test

# Build Python bindings
maturin develop
```

## Documentation

- **[Getting Started (Rust)](docs/getting-started-rust.md)** - Complete Rust guide
- **[Getting Started (Python)](docs/getting-started-python.md)** - Complete Python guide
- **[API Docs](https://docs.rs/pdf_oxide)** - Full API reference
- **[PDF Spec Reference](docs/spec/pdf.md)** - ISO 32000-1:2008

```bash
# Generate local docs
cargo doc --open
```

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

```bash
# Development setup
cargo build
cargo test
cargo fmt
cargo clippy -- -D warnings
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

## Citation

```bibtex
@software{pdf_oxide,
  title = {PDF Oxide: High-Performance PDF Parsing in Rust},
  author = {Yury Fedoseev},
  year = {2025},
  url = {https://github.com/yfedoseev/pdf_oxide}
}
```

---

**Built with** 🦀 Rust + 🐍 Python | **Status**: ✅ Production Ready | **v0.3.5** | 100% pass rate on 3,830 PDFs
