use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "pdf-oxide",
    version,
    about = "Fast, local PDF processing",
    long_about = "pdf-oxide — the fastest PDF toolkit.\nRun with no arguments for interactive REPL mode."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Output file path (defaults to stdout for text outputs)
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Page range, e.g. "1-5", "1,3,7", "1-3,7,10-12"
    #[arg(short, long, global = true)]
    pub pages: Option<String>,

    /// Show verbose output with timing
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress all non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output as JSON
    #[arg(short, long, global = true)]
    pub json: bool,

    /// Password for encrypted PDFs
    #[arg(long, global = true)]
    pub password: Option<String>,

    /// Skip the banner in REPL mode
    #[arg(long, global = true)]
    pub no_banner: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Extract plain text from a PDF
    Text {
        /// Input PDF file
        file: PathBuf,
    },

    /// Convert PDF to Markdown
    Markdown {
        /// Input PDF file
        file: PathBuf,
    },

    /// Convert PDF to HTML
    Html {
        /// Input PDF file
        file: PathBuf,
    },

    /// Show PDF metadata and page count
    Info {
        /// Input PDF file
        file: PathBuf,
    },

    /// Merge multiple PDFs into one
    Merge {
        /// Input PDF files (first file is the base)
        #[arg(required = true, num_args = 2..)]
        files: Vec<PathBuf>,
    },

    /// Split a PDF into individual pages
    Split {
        /// Input PDF file
        file: PathBuf,
    },

    /// Create a PDF from Markdown, HTML, or plain text
    Create {
        /// Input source file
        file: PathBuf,

        /// Input format
        #[arg(long, value_parser = ["markdown", "html", "text"])]
        from: String,
    },

    /// Compress and optimize a PDF
    Compress {
        /// Input PDF file
        file: PathBuf,
    },

    /// Encrypt a PDF with a password (placeholder — coming in v0.4.0)
    Encrypt {
        /// Input PDF file
        file: PathBuf,
    },

    /// Decrypt a password-protected PDF
    Decrypt {
        /// Input PDF file
        file: PathBuf,

        /// Password to decrypt
        #[arg(long)]
        password: String,
    },

    /// Search for text in a PDF
    Search {
        /// Input PDF file
        file: PathBuf,

        /// Search pattern (regex supported)
        pattern: String,

        /// Case-insensitive search
        #[arg(short, long)]
        ignore_case: bool,
    },

    /// Extract images from a PDF
    Images {
        /// Input PDF file
        file: PathBuf,
    },
}
