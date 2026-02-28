use crate::search::{SearchOptions, TextSearcher};
use std::path::Path;

pub fn run(
    file: &Path,
    pattern: &str,
    ignore_case: bool,
    pages: Option<&str>,
    password: Option<&str>,
    json: bool,
) -> crate::Result<()> {
    let mut doc = super::open_doc(file, password)?;
    let page_count = doc.page_count()?;

    let page_range = if let Some(ranges) = pages {
        let indices = super::resolve_pages(Some(ranges), page_count)?;
        let min = *indices.iter().min().unwrap_or(&0);
        let max = *indices.iter().max().unwrap_or(&0);
        Some((min, max))
    } else {
        None
    };

    let options = SearchOptions {
        case_insensitive: ignore_case,
        page_range,
        ..Default::default()
    };

    let results = TextSearcher::search(&mut doc, pattern, &options)?;

    if json {
        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "page": r.page + 1,
                    "text": r.text,
                    "start_index": r.start_index,
                    "end_index": r.end_index,
                })
            })
            .collect();
        let json_out = serde_json::json!({
            "file": file.display().to_string(),
            "pattern": pattern,
            "matches": results.len(),
            "results": json_results,
        });
        super::write_output(&serde_json::to_string_pretty(&json_out).unwrap(), None)?;
    } else if results.is_empty() {
        eprintln!("No matches found for '{pattern}'");
    } else {
        eprintln!("Found {} match(es) for '{pattern}':", results.len());
        for r in &results {
            println!("  Page {}: \"{}\"", r.page + 1, r.text);
        }
    }

    Ok(())
}
