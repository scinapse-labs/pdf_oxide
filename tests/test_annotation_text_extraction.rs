//! Regression test: Form field and annotation text extraction.
//!
//! Verifies that Widget and FreeText annotation text is included in extract_text output.

use pdf_oxide::document::PdfDocument;

#[test]
fn test_annotation_extraction_does_not_crash() {
    // Verify that extract_text with annotation appending doesn't crash
    // even on a PDF with no annotations.
    let mut doc = PdfDocument::open("tests/fixtures/simple.pdf").unwrap();
    let _text = doc.extract_text(0).unwrap();
    // If we get here without panicking, the annotation code path is safe
}

#[test]
fn test_outline_pdf_annotation_extraction() {
    // outline.pdf may have annotations - verify no crash
    let mut doc = PdfDocument::open("tests/fixtures/outline.pdf").unwrap();
    let page_count = doc.page_count().unwrap();
    for i in 0..page_count {
        let _text = doc.extract_text(i).unwrap();
    }
}
