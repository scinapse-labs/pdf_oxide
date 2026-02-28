use crate::editor::{DocumentEditor, EditableDocument};
use std::path::Path;

pub fn run(
    files: &[std::path::PathBuf],
    output: Option<&Path>,
) -> crate::Result<()> {
    if files.len() < 2 {
        return Err(crate::Error::InvalidOperation(
            "Merge requires at least 2 PDF files".to_string(),
        ));
    }

    let mut editor = DocumentEditor::open(&files[0])?;

    for source in &files[1..] {
        let pages_added = editor.merge_from(source)?;
        eprintln!("Merged {} pages from {}", pages_added, source.display());
    }

    let out_path = output.unwrap_or_else(|| Path::new("merged.pdf"));
    editor.save(out_path)?;
    eprintln!("Saved to {}", out_path.display());

    Ok(())
}
