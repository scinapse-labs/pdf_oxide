use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use crate::PdfDocument;
use super::colors;

struct ReplState {
    current_doc: Option<PdfDocument>,
    current_file: Option<PathBuf>,
    password: Option<String>,
    json: bool,
}

impl ReplState {
    fn prompt(&self) -> String {
        if let Some(ref f) = self.current_file {
            let name = f.file_name().and_then(|s| s.to_str()).unwrap_or("?");
            format!("pdf-oxide [{}]> ", name)
        } else {
            "pdf-oxide> ".to_string()
        }
    }

    fn ensure_doc(&mut self) -> crate::Result<&mut PdfDocument> {
        self.current_doc
            .as_mut()
            .ok_or_else(|| crate::Error::InvalidOperation(
                "No PDF loaded. Use 'open <file>' first.".to_string(),
            ))
    }
}

pub fn enter(
    no_banner: bool,
    password: Option<String>,
    json: bool,
    _verbose: bool,
) -> crate::Result<()> {
    if !no_banner {
        super::banner::print_banner();
        eprintln!("Type {} for commands, {} to quit.", colors::bold("help"), colors::bold("exit"));
        eprintln!();
    }

    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let mut state = ReplState {
        current_doc: None,
        current_file: None,
        password,
        json,
    };
    let mut line = String::new();

    loop {
        eprint!("{}", colors::rust_orange(&state.prompt()));
        std::io::stderr().flush().ok();

        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF (Ctrl+D)
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", colors::error(&format!("Read error: {e}")));
                break;
            }
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.splitn(2, char::is_whitespace).collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        let result = match cmd.as_str() {
            "exit" | "quit" | "q" | "bye" => break,
            "help" | "?" | "h" => {
                print_help();
                Ok(())
            }
            "open" | "o" | "load" => cmd_open(&mut state, args),
            "close" | "c" => cmd_close(&mut state),
            "text" | "t" => cmd_text(&mut state, args),
            "markdown" | "md" => cmd_markdown(&mut state, args),
            "html" => cmd_html(&mut state, args),
            "info" | "i" => cmd_info(&mut state, args),
            "search" | "s" | "find" | "grep" => cmd_search(&mut state, args),
            "images" | "img" => cmd_images(&mut state, args),
            "pages" | "p" => cmd_pages(&mut state),
            _ => {
                eprintln!("Unknown command: '{}'. Type 'help' for available commands.", cmd);
                Ok(())
            }
        };

        if let Err(e) = result {
            eprintln!("{}", colors::error(&format!("Error: {e}")));
        }
    }

    Ok(())
}

fn print_help() {
    eprintln!("Commands:");
    eprintln!("  open|o <file>      Load a PDF file");
    eprintln!("  close|c            Close the current PDF");
    eprintln!("  info|i [file]      Show PDF metadata");
    eprintln!("  text|t [file]      Extract plain text");
    eprintln!("  markdown|md [file] Convert to Markdown");
    eprintln!("  html [file]        Convert to HTML");
    eprintln!("  search|s <pattern> Search text (also: find, grep)");
    eprintln!("  images|img [file]  Extract images to current directory");
    eprintln!("  pages|p            Show page count of current PDF");
    eprintln!("  help|h|?           Show this help message");
    eprintln!("  exit|quit|q        Exit the REPL (also: bye, Ctrl+D)");
}

fn cmd_open(state: &mut ReplState, args: &str) -> crate::Result<()> {
    if args.is_empty() {
        return Err(crate::Error::InvalidOperation("Usage: open <file>".to_string()));
    }
    let path = PathBuf::from(args);
    let mut doc = PdfDocument::open(&path)?;
    if let Some(ref pw) = state.password {
        doc.authenticate(pw.as_bytes())?;
    }
    let pages = doc.page_count()?;
    state.current_doc = Some(doc);
    state.current_file = Some(path.clone());
    eprintln!("Opened {} ({} pages)", path.display(), pages);
    Ok(())
}

fn cmd_close(state: &mut ReplState) -> crate::Result<()> {
    if state.current_doc.is_some() {
        let name = state
            .current_file
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        state.current_doc = None;
        state.current_file = None;
        eprintln!("Closed {name}");
    } else {
        eprintln!("No PDF is currently open.");
    }
    Ok(())
}

fn with_doc(state: &mut ReplState, args: &str, f: impl FnOnce(&mut PdfDocument) -> crate::Result<()>) -> crate::Result<()> {
    if args.is_empty() {
        let doc = state.ensure_doc()?;
        f(doc)
    } else {
        let mut doc = PdfDocument::open(args)?;
        if let Some(ref pw) = state.password {
            doc.authenticate(pw.as_bytes())?;
        }
        f(&mut doc)
    }
}

fn cmd_text(state: &mut ReplState, args: &str) -> crate::Result<()> {
    with_doc(state, args, |doc| {
        let page_count = doc.page_count()?;
        for i in 0..page_count {
            let text = doc.extract_text(i)?;
            println!("{text}");
        }
        Ok(())
    })
}

fn cmd_markdown(state: &mut ReplState, args: &str) -> crate::Result<()> {
    with_doc(state, args, |doc| {
        let page_count = doc.page_count()?;
        let options = crate::converters::ConversionOptions::default();
        for i in 0..page_count {
            let md = doc.to_markdown(i, &options)?;
            println!("{md}");
        }
        Ok(())
    })
}

fn cmd_html(state: &mut ReplState, args: &str) -> crate::Result<()> {
    with_doc(state, args, |doc| {
        let page_count = doc.page_count()?;
        let options = crate::converters::ConversionOptions::default();
        for i in 0..page_count {
            let html = doc.to_html(i, &options)?;
            println!("{html}");
        }
        Ok(())
    })
}

fn cmd_info(state: &mut ReplState, args: &str) -> crate::Result<()> {
    if !args.is_empty() {
        super::commands::info::run(Path::new(args), state.password.as_deref(), state.json)
    } else {
        let path = state
            .current_file
            .as_ref()
            .ok_or_else(|| crate::Error::InvalidOperation(
                "No PDF loaded. Use 'open <file>' or provide a file path.".to_string(),
            ))?
            .clone();
        super::commands::info::run(&path, state.password.as_deref(), state.json)
    }
}

fn cmd_search(state: &mut ReplState, args: &str) -> crate::Result<()> {
    if args.is_empty() {
        return Err(crate::Error::InvalidOperation("Usage: search <pattern>".to_string()));
    }
    let doc = state.ensure_doc()?;
    let options = crate::search::SearchOptions::default();
    let results = crate::search::TextSearcher::search(doc, args, &options)?;

    if results.is_empty() {
        eprintln!("No matches found for '{args}'");
    } else {
        eprintln!("Found {} match(es):", results.len());
        for r in &results {
            println!("  Page {}: \"{}\"", r.page + 1, r.text);
        }
    }
    Ok(())
}

fn cmd_images(state: &mut ReplState, args: &str) -> crate::Result<()> {
    if !args.is_empty() {
        super::commands::images::run(
            Path::new(args),
            None,
            Some(Path::new(".")),
            state.password.as_deref(),
            state.json,
        )
    } else {
        let path = state
            .current_file
            .as_ref()
            .ok_or_else(|| crate::Error::InvalidOperation(
                "No PDF loaded. Use 'open <file>' or provide a file path.".to_string(),
            ))?
            .clone();
        super::commands::images::run(
            &path,
            None,
            Some(Path::new(".")),
            state.password.as_deref(),
            state.json,
        )
    }
}

fn cmd_pages(state: &mut ReplState) -> crate::Result<()> {
    let doc = state.ensure_doc()?;
    let count = doc.page_count()?;
    println!("{count} pages");
    Ok(())
}
