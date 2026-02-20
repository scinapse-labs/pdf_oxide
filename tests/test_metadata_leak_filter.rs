//! Regression test: Metadata leaks into extracted text.
//!
//! Verifies that CalRGB/ICCBased color space data doesn't appear in text output.

use pdf_oxide::document::PdfDocument;

#[test]
fn test_no_metadata_in_outline_pdf() {
    let mut doc = PdfDocument::open("tests/fixtures/outline.pdf").unwrap();
    let page_count = doc.page_count().unwrap();

    for i in 0..page_count {
        let text = doc.extract_text(i).unwrap();
        assert!(!text.contains("WhitePoint"), "Page {}: contains WhitePoint", i);
        assert!(!text.contains("BlackPoint"), "Page {}: contains BlackPoint", i);
        assert!(!text.contains("/CalRGB"), "Page {}: contains /CalRGB", i);
        assert!(!text.contains("/ICCBased"), "Page {}: contains /ICCBased", i);
    }
}
