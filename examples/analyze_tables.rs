/// Analyze table extraction quality across markdown, text, and HTML outputs
/// Usage: cargo run --release --example analyze_tables <pdf_dir_or_file>
use pdf_oxide::PdfDocument;
use pdf_oxide::converters::ConversionOptions;
use std::path::{Path, PathBuf};

fn analyze_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let filename = path.file_name().unwrap().to_string_lossy();
    let data = std::fs::read(path)?;
    let mut doc = match PdfDocument::open_from_bytes(data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("  SKIP {}: {}", filename, e);
            return Ok(());
        }
    };

    let page_count = doc.page_count().unwrap_or(0);
    let max_pages = page_count.min(5); // Analyze first 5 pages

    println!("\n{}", "=".repeat(80));
    println!("FILE: {} ({} pages, analyzing first {})", filename, page_count, max_pages);
    println!("{}", "=".repeat(80));

    let options: ConversionOptions = ConversionOptions::default()
        .with_default_table_detection();

    let options_no_tables: ConversionOptions = ConversionOptions::default();

    for page_idx in 0..max_pages {
        println!("\n--- Page {} ---", page_idx + 1);

        // Plain text
        match doc.extract_text(page_idx) {
            Ok(text) => {
                let lines: Vec<&str> = text.lines().collect();
                let has_pipe = text.contains('|');
                let has_tab_alignment = text.lines().any(|l| l.contains('\t'));
                let has_grid_chars = text.contains('+') && text.contains('-') && text.contains('|');
                println!("  TEXT: {} chars, {} lines | pipe:{} tab:{} grid:{}",
                    text.len(), lines.len(), has_pipe, has_tab_alignment, has_grid_chars);

                // Show lines that look tabular (multiple spaces separating values)
                let tabular_lines: Vec<&str> = lines.iter()
                    .filter(|l| {
                        let parts: Vec<&str> = l.split("   ").filter(|s| !s.is_empty()).collect();
                        parts.len() >= 3
                    })
                    .copied()
                    .collect();
                if !tabular_lines.is_empty() {
                    println!("  TEXT tabular-looking lines: {}", tabular_lines.len());
                    for line in tabular_lines.iter().take(5) {
                        println!("    | {}", line.trim());
                    }
                    if tabular_lines.len() > 5 {
                        println!("    ... ({} more)", tabular_lines.len() - 5);
                    }
                }
            }
            Err(e) => println!("  TEXT: ERROR - {}", e),
        }

        // Markdown with table detection
        match doc.to_markdown(page_idx, &options) {
            Ok(md) => {
                let has_md_table = md.contains("| ") && md.contains(" |") && md.contains("---");
                let table_rows = md.lines().filter(|l| l.starts_with('|') && l.ends_with('|')).count();
                let has_heading = md.contains("# ");
                println!("  MARKDOWN (tables=on): {} chars | md_table:{} rows:{} heading:{}",
                    md.len(), has_md_table, table_rows, has_heading);

                if has_md_table {
                    // Show first table
                    let mut in_table = false;
                    let mut shown = 0;
                    for line in md.lines() {
                        if line.starts_with('|') && line.ends_with('|') {
                            if !in_table {
                                println!("  MARKDOWN table sample:");
                                in_table = true;
                            }
                            if shown < 6 {
                                println!("    {}", line);
                                shown += 1;
                            }
                        } else if in_table {
                            if shown >= 6 {
                                println!("    ... ({} total rows)", table_rows);
                            }
                            break;
                        }
                    }
                }
            }
            Err(e) => println!("  MARKDOWN (tables=on): ERROR - {}", e),
        }

        // Markdown without table detection for comparison
        match doc.to_markdown(page_idx, &options_no_tables) {
            Ok(md) => {
                let has_md_table = md.contains("| ") && md.contains(" |") && md.contains("---");
                let table_rows = md.lines().filter(|l| l.starts_with('|') && l.ends_with('|')).count();
                println!("  MARKDOWN (tables=off): {} chars | md_table:{} rows:{}",
                    md.len(), has_md_table, table_rows);
            }
            Err(e) => println!("  MARKDOWN (tables=off): ERROR - {}", e),
        }

        // HTML
        match doc.to_html(page_idx, &options) {
            Ok(html) => {
                let has_html_table = html.contains("<table");
                let td_count = html.matches("<td").count();
                let th_count = html.matches("<th").count();
                let tr_count = html.matches("<tr").count();
                println!("  HTML: {} chars | <table>:{} <tr>:{} <td>:{} <th>:{}",
                    html.len(), has_html_table, tr_count, td_count, th_count);

                if has_html_table {
                    // Show first few rows of first table
                    if let Some(start) = html.find("<table") {
                        let end = html.len().min(start + 800);
                        let table_snippet = &html[start..end];
                        println!("  HTML table sample (first 800 chars):");
                        for line in table_snippet.lines().take(15) {
                            println!("    {}", line.trim());
                        }
                    }
                }
            }
            Err(e) => println!("  HTML: ERROR - {}", e),
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf_path_or_directory>", args[0]);
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[1]);
    let mut pdfs: Vec<PathBuf> = Vec::new();

    if path.is_dir() {
        for entry in std::fs::read_dir(&path)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().map(|e| e == "pdf").unwrap_or(false) {
                pdfs.push(p);
            }
        }
        pdfs.sort();
    } else {
        pdfs.push(path);
    }

    println!("Analyzing {} PDF(s) for table extraction quality...\n", pdfs.len());

    for pdf in &pdfs {
        if let Err(e) = analyze_pdf(pdf) {
            eprintln!("ERROR processing {}: {}", pdf.display(), e);
        }
    }

    println!("\n\n{}", "=".repeat(80));
    println!("ANALYSIS COMPLETE: {} PDFs processed", pdfs.len());
    println!("{}", "=".repeat(80));

    Ok(())
}
