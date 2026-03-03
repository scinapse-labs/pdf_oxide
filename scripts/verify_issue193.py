from pdf_oxide import PdfDocument
import os

def verify_bug():
    pdf_path = "tests/fixtures/1.pdf"
    if not os.path.exists(pdf_path):
        print(f"Error: {pdf_path} not found")
        return

    print("--- Scenario 1: Spans then Chars ---")
    doc = PdfDocument(pdf_path)
    spans1 = doc.extract_spans(0)
    spans2 = doc.extract_spans(0)
    chars1 = doc.extract_chars(0)
    chars2 = doc.extract_chars(0)
    
    print(f"Spans1: {len(spans1)}, Spans2: {len(spans2)}")
    print(f"Chars1: {len(chars1)}, Chars2: {len(chars2)}")
    
    assert len(spans1) == len(spans2) and len(spans1) > 0, "Spans inconsistency"
    assert len(chars1) == len(chars2) and len(chars1) > 0, "Chars inconsistency"

    print("\n--- Scenario 2: Chars then Spans ---")
    doc = PdfDocument(pdf_path)
    chars1 = doc.extract_chars(0)
    chars2 = doc.extract_chars(0)
    spans1 = doc.extract_spans(0)
    spans2 = doc.extract_spans(0)
    
    print(f"Chars1: {len(chars1)}, Chars2: {len(chars2)}")
    print(f"Spans1: {len(spans1)}, Spans2: {len(spans2)}")
    
    assert len(chars1) == len(chars2) and len(chars1) > 0, "Chars inconsistency"
    assert len(spans1) == len(spans2) and len(spans1) > 0, "Spans inconsistency"
    
    print("\n✅ Issue #193 verified: Extractions are consistent!")

if __name__ == "__main__":
    verify_bug()
