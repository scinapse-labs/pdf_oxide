use std::path::Path;

pub fn run(
    file: &Path,
    pages: Option<&str>,
    output: Option<&Path>,
    password: Option<&str>,
    json: bool,
) -> crate::Result<()> {
    let mut doc = super::open_doc(file, password)?;
    let page_count = doc.page_count()?;
    let page_indices = super::resolve_pages(pages, page_count)?;

    let out_dir = output.unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(out_dir)?;

    let stem = file
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("img");

    let mut total_images = 0;
    let mut all_images = Vec::new();

    for &page_idx in &page_indices {
        let prefix = format!("{stem}_p{}", page_idx + 1);
        let images = doc.extract_images_to_files(
            page_idx,
            out_dir,
            Some(&prefix),
            Some(total_images),
        )?;
        total_images += images.len();
        all_images.extend(images);
    }

    if json {
        let json_images: Vec<serde_json::Value> = all_images
            .iter()
            .map(|img| {
                serde_json::json!({
                    "width": img.width,
                    "height": img.height,
                    "format": format!("{:?}", img.format),
                })
            })
            .collect();
        let json_out = serde_json::json!({
            "file": file.display().to_string(),
            "output_dir": out_dir.display().to_string(),
            "images_extracted": total_images,
            "images": json_images,
        });
        super::write_output(&serde_json::to_string_pretty(&json_out).unwrap(), None)?;
    } else {
        eprintln!(
            "Extracted {} image(s) to {}",
            total_images,
            out_dir.display()
        );
    }

    Ok(())
}
