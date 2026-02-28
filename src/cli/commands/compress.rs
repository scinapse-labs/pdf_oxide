use crate::editor::{DocumentEditor, EditableDocument, SaveOptions};
use std::path::Path;

pub fn run(
    file: &Path,
    output: Option<&Path>,
    password: Option<&str>,
) -> crate::Result<()> {
    // Note: DocumentEditor doesn't support authenticate(); encrypted PDFs should
    // be decrypted via PdfDocument first. For compress, we open directly.
    let _ = password; // Password handling for DocumentEditor not yet supported
    let mut editor = DocumentEditor::open(file)?;

    let out_path = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("pdf");
        Path::new(&format!("{stem}_compressed.{ext}")).to_path_buf()
    });

    let original_size = std::fs::metadata(file)
        .map(|m| m.len())
        .unwrap_or(0);

    editor.save_with_options(&out_path, SaveOptions {
        compress: true,
        garbage_collect: true,
        linearize: true,
        ..Default::default()
    })?;

    let new_size = std::fs::metadata(&out_path)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!(
        "Compressed {} -> {} ({} -> {} bytes)",
        file.display(),
        out_path.display(),
        original_size,
        new_size,
    );

    Ok(())
}
