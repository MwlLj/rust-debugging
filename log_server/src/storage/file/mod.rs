extern crate chrono;

use std::io::prelude::*;
use chrono::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use std::fs::OpenOptions;
use std::fs::File;
use std::fs::DirBuilder;
use std::io::Seek;
use std::io::BufWriter;
use std::io::SeekFrom;

use super::IStorage;

pub struct CFile {
}

const log_file_max_byte_len: u64 = 5242880;

impl CFile {
    fn now(&self) -> String {
        let dt = Local::now();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn createDir(&self, path: &str) {
    	if Path::new(path).exists() {
    		return ();
    	}
    	if let Ok(_) = DirBuilder::new().create(path) {
    		return ();
    	}
    }

    pub fn walkFiles<F>(&self, dirName: &str, mut callBack: F) -> Result<i32, &str>
        where F: FnMut(String) {
        let mut paths = match fs::read_dir(dirName) {
            Err(why) => return Err("read dir error"),
            Ok(paths) => paths
        };
        for path in paths {
            if let Ok(path) = path {
                let path = path.path();
                let p = Path::new(&path);
                let path = p.to_str();
                if let Some(ref path) = path {
                    if p.is_file() {
                        // file
                        callBack(path.to_string());
                    }
                }
            }
        }
        Ok(0)
    }

    fn findFile<'a>(&self, dir: &'a str) -> Result<PathBuf, &str> {
    	let mut v = Vec::new();
    	if let Ok(_) = self.walkFiles(dir, |fileName: String| {
    		v.push(fileName);
    	}) {
    	}
    	if let Some(max) = v.iter().max() {
	    	if let Ok(meta) = fs::metadata(max.as_str()) {
	    		let len = meta.len();
	    		if len > log_file_max_byte_len {
	    			if let Some(name) = Path::new(max.as_str()).file_stem() {
	    				if let Some(name) = name.to_str() {
		    				if let Ok(value) = name.parse::<i32>() {
		    					let value = value + 1;
		    					return Ok(Path::new(dir).join(value.to_string() + ".log"));
		    				}
	    				}
	    			}
	    		} else {
	    			let mut buf = PathBuf::new();
	    			buf.push(max);
	    			return Ok(buf);
	    		}
	    	}
    	} else {
    		return Ok(Path::new(dir).join("1.log".to_string()));
    	}
    	Err("not found")
    }

    fn write(&self, root: &str, contentType: &str, content: &str) -> std::io::Result<()> {
    	self.createDir(root);
    	if let Ok(path) = self.findFile(root) {
	    	println!("{:?}", &path);
	        let f = OpenOptions::new().append(true).create(true).open(path)?;
	        let mut writer = BufWriter::new(f);
	        writer.write("[".as_bytes());
	        writer.write(contentType.as_bytes());
	        writer.write("] [".as_bytes());
	        writer.write(self.now().as_bytes());
	        writer.write("] ".as_bytes());
	        writer.write(content.as_bytes())?;
	        writer.flush()?;
	    }
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

