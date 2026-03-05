//! Plain text output converter.
//!
//! Converts ordered text spans to plain text format.

use crate::error::Result;
use crate::pipeline::{OrderedTextSpan, TextPipelineConfig};
use crate::structure::table_extractor::ExtractedTable;
use crate::text::HyphenationHandler;

use super::OutputConverter;

/// Plain text output converter.
///
/// Converts ordered text spans to plain text, preserving paragraph structure
/// but removing all formatting.
pub struct PlainTextConverter {
    /// Line spacing threshold ratio for paragraph detection.
    paragraph_gap_ratio: f32,
}

impl PlainTextConverter {
    /// Create a new plain text converter with default settings.
    pub fn new() -> Self {
        Self {
            paragraph_gap_ratio: 1.5,
        }
    }

    /// Detect paragraph breaks between spans based on vertical spacing.
    fn is_paragraph_break(&self, current: &OrderedTextSpan, previous: &OrderedTextSpan) -> bool {
        let line_height = current.span.font_size.max(previous.span.font_size);
        let gap = (previous.span.bbox.y - current.span.bbox.y).abs();
        gap > line_height * self.paragraph_gap_ratio
    }
}

impl Default for PlainTextConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputConverter for PlainTextConverter {
    fn convert(&self, spans: &[OrderedTextSpan], config: &TextPipelineConfig) -> Result<String> {
        self.render_spans(spans, &[], config)
    }

    fn convert_with_tables(
        &self,
        spans: &[OrderedTextSpan],
        tables: &[ExtractedTable],
        config: &TextPipelineConfig,
    ) -> Result<String> {
        self.render_spans(spans, tables, config)
    }

    fn name(&self) -> &'static str {
        "PlainTextConverter"
    }

    fn mime_type(&self) -> &'static str {
        "text/plain"
    }
}

impl PlainTextConverter {
    /// Check if a span's bbox overlaps with any table region.
    fn span_in_table(&self, span: &OrderedTextSpan, tables: &[ExtractedTable]) -> Option<usize> {
        let sx = span.span.bbox.x;
        let sy = span.span.bbox.y;

        for (i, table) in tables.iter().enumerate() {
            if let Some(ref bbox) = table.bbox {
                let tolerance = 2.0;
                if sx >= bbox.x - tolerance
                    && sx <= bbox.x + bbox.width + tolerance
                    && sy >= bbox.y - tolerance
                    && sy <= bbox.y + bbox.height + tolerance
                {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Render an ExtractedTable as tab-delimited plain text.
    fn render_table_text(table: &ExtractedTable) -> String {
        let mut output = String::new();
        for row in &table.rows {
            let cells: Vec<&str> = row.cells.iter().map(|c| c.text.trim()).collect();
            output.push_str(&cells.join("\t"));
            output.push('\n');
        }
        output
    }

    /// Core rendering logic.
    fn render_spans(
        &self,
        spans: &[OrderedTextSpan],
        tables: &[ExtractedTable],
        config: &TextPipelineConfig,
    ) -> Result<String> {
        if spans.is_empty() && tables.is_empty() {
            return Ok(String::new());
        }

        let mut sorted: Vec<_> = spans.iter().collect();
        sorted.sort_by_key(|s| s.reading_order);

        let mut tables_rendered = vec![false; tables.len()];
        let mut result = String::new();
        let mut prev_span: Option<&OrderedTextSpan> = None;

        for span in &sorted {
            // Check if span is in a table region
            if !tables.is_empty() {
                if let Some(table_idx) = self.span_in_table(span, tables) {
                    if !tables_rendered[table_idx] {
                        // Add blank line before table
                        if !result.is_empty() && !result.ends_with("\n\n") {
                            if !result.ends_with('\n') {
                                result.push('\n');
                            }
                            result.push('\n');
                        }
                        result.push_str(&Self::render_table_text(&tables[table_idx]));
                        tables_rendered[table_idx] = true;
                        prev_span = None;
                    }
                    continue;
                }
            }

            if let Some(prev) = prev_span {
                if self.is_paragraph_break(span, prev) {
                    result.push_str("\n\n");
                } else {
                    let same_line =
                        (span.span.bbox.y - prev.span.bbox.y).abs() < span.span.font_size * 0.5;
                    if !same_line {
                        result.push(' ');
                    }
                }
            }

            result.push_str(&span.span.text);
            prev_span = Some(span);
        }

        // Render any unmatched tables
        for (i, table) in tables.iter().enumerate() {
            if !tables_rendered[i] && !table.is_empty() {
                if !result.is_empty() && !result.ends_with("\n\n") {
                    if !result.ends_with('\n') {
                        result.push('\n');
                    }
                    result.push('\n');
                }
                result.push_str(&Self::render_table_text(table));
            }
        }

        if !result.ends_with('\n') {
            result.push('\n');
        }

        if config.enable_hyphenation_reconstruction {
            let handler = HyphenationHandler::new();
            result = handler.process_text(&result);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;
    use crate::layout::{Color, FontWeight, TextSpan};

    fn make_span(text: &str, x: f32, y: f32) -> OrderedTextSpan {
        OrderedTextSpan::new(
            TextSpan {
                artifact_type: None,
                text: text.to_string(),
                bbox: Rect::new(x, y, 50.0, 12.0),
                font_name: "Test".to_string(),
                font_size: 12.0,
                font_weight: FontWeight::Normal,
                is_italic: false,
                color: Color::black(),
                mcid: None,
                sequence: 0,
                offset_semantic: false,
                split_boundary_before: false,
                char_spacing: 0.0,
                word_spacing: 0.0,
                horizontal_scaling: 100.0,
                primary_detected: false,
            },
            0,
        )
    }

    #[test]
    fn test_empty_spans() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();
        let result = converter.convert(&[], &config).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_line() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();
        let spans = vec![make_span("Hello world", 0.0, 100.0)];
        let result = converter.convert(&spans, &config).unwrap();
        assert_eq!(result, "Hello world\n");
    }

    #[test]
    fn test_paragraph_break() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();
        let mut spans = vec![
            make_span("First paragraph", 0.0, 100.0),
            make_span("Second paragraph", 0.0, 50.0), // Large gap indicates new paragraph
        ];
        spans[1].reading_order = 1;

        let result = converter.convert(&spans, &config).unwrap();
        assert!(result.contains("\n\n"));
    }

    // ============================================================================
    // Table rendering tests
    // ============================================================================

    use crate::structure::table_extractor::{TableCell, TableRow};

    #[test]
    fn test_render_table_text_basic() {
        let mut table = ExtractedTable::new();
        let mut row1 = TableRow::new(false);
        row1.add_cell(TableCell::new("A".to_string(), false));
        row1.add_cell(TableCell::new("B".to_string(), false));
        table.add_row(row1);

        let mut row2 = TableRow::new(false);
        row2.add_cell(TableCell::new("C".to_string(), false));
        row2.add_cell(TableCell::new("D".to_string(), false));
        table.add_row(row2);

        let result = PlainTextConverter::render_table_text(&table);
        assert_eq!(result, "A\tB\nC\tD\n");
    }

    #[test]
    fn test_render_table_text_empty() {
        let table = ExtractedTable::new();
        let result = PlainTextConverter::render_table_text(&table);
        assert_eq!(result, "");
    }

    #[test]
    fn test_render_table_text_trims_whitespace() {
        let mut table = ExtractedTable::new();
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("  padded  ".to_string(), false));
        table.add_row(row);

        let result = PlainTextConverter::render_table_text(&table);
        assert_eq!(result, "padded\n");
    }

    #[test]
    fn test_convert_with_tables_renders_tab_delimited() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("X".to_string(), false));
        row.add_cell(TableCell::new("Y".to_string(), false));
        table.add_row(row);

        let result = converter
            .convert_with_tables(&[], &[table], &config)
            .unwrap();

        assert!(result.contains("X\tY"), "Should contain tab-delimited cells: {:?}", result);
    }

    #[test]
    fn test_convert_with_tables_mixed_content() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();

        let mut span_before = make_span("Before", 10.0, 200.0);
        span_before.reading_order = 0;

        let mut span_in_table = make_span("Inside", 50.0, 70.0);
        span_in_table.reading_order = 1;

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));
        let mut row = TableRow::new(false);
        row.add_cell(TableCell::new("Cell".to_string(), false));
        table.add_row(row);

        let result = converter
            .convert_with_tables(&[span_before, span_in_table], &[table], &config)
            .unwrap();

        assert!(result.contains("Before"), "Should contain text before table");
        assert!(result.contains("Cell"), "Should contain table cell");
        assert!(!result.contains("Inside"), "Should exclude span inside table region");
    }

    #[test]
    fn test_convert_with_tables_no_tables_same_as_convert() {
        let converter = PlainTextConverter::new();
        let config = TextPipelineConfig::default();
        let spans = vec![make_span("Hello", 0.0, 100.0)];

        let result_convert = converter.convert(&spans, &config).unwrap();
        let result_with_tables = converter.convert_with_tables(&spans, &[], &config).unwrap();

        assert_eq!(result_convert, result_with_tables);
    }

    #[test]
    fn test_span_in_table_plain_text() {
        let converter = PlainTextConverter::new();

        let mut table = ExtractedTable::new();
        table.bbox = Some(Rect::new(10.0, 50.0, 200.0, 100.0));

        let inside = make_span("inside", 50.0, 70.0);
        let outside = make_span("outside", 500.0, 500.0);

        assert_eq!(converter.span_in_table(&inside, &[table.clone()]), Some(0));
        assert_eq!(converter.span_in_table(&outside, &[table]), None);
    }

    #[test]
    fn test_span_in_table_no_bbox() {
        let converter = PlainTextConverter::new();

        let table = ExtractedTable::new(); // No bbox
        let span = make_span("text", 50.0, 70.0);

        assert_eq!(converter.span_in_table(&span, &[table]), None);
    }
}
