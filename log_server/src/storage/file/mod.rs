extern crate chrono;

use std::io::prelude::*;
use chrono::prelude::*;
use std::fs::OpenOptions;
use std::fs::File;
use std::io::Seek;
use std::io::BufWriter;
use std::io::SeekFrom;

use super::IStorage;

pub struct CFile {
}

impl CFile {
    fn now(&self) -> String {
        let dt = Local::now();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }
    fn write(&self, path: &str, contentType: &str, content: &str) -> std::io::Result<()> {
        let mut path = path.to_string();
        path.insert_str(path.len(), ".log");
        let f = OpenOptions::new().append(true).create(true).open(path)?;
        let mut writer = BufWriter::new(f);
        writer.write("[".as_bytes());
        writer.write(contentType.as_bytes());
        writer.write("] [".as_bytes());
        writer.write(self.now().as_bytes());
        writer.write("] ".as_bytes());
        writer.write(content.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

impl IStorage for CFile {
    fn message(&self, path: &str, content: &str) -> std::io::Result<()> {
        self.write(path, "message", content)
    }
    fn error(&self, path: &str, content: &str) -> std::io::Result<()> {
        self.write(path, "error", content)
    }
}

impl CFile {
    pub fn new() -> CFile {
        CFile{}
    }
}

