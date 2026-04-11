use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SessionEntry {
    pub timestamp: String,
    pub url: Option<String>,
    pub query: Option<String>,
    pub source_type: String,
    pub domain: Option<String>,
    pub tokens: usize,
    pub bytes: usize,
    pub cache_hit: bool,
    pub mode: String,
    pub keywords_found: Vec<String>,
    pub output_chars: usize,
    pub success: bool,
    pub exit_code: i32,
    pub error: Option<String>,
}

pub fn append_jsonl(path: &Path, entry: &SessionEntry) -> io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, entry)
        .map_err(|e| io::Error::other(format!("JSON serialization failed: {e}")))?;
    file.write_all(b"\n")?;
    Ok(())
}
