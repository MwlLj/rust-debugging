extern crate chrono;

use std::io::prelude::*;
use chrono::prelude::*;
use std::path;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use std::fs::OpenOptions;
use std::fs::File;
use std::fs::DirBuilder;
use std::io::Seek;
use std::io::BufWriter;
use std::io::SeekFrom;
use std::ffi::OsString;

use super::IStorage;

pub struct CFile {
    maxSecond: i64,
    date: Option<Date<Local>>
}

const log_file_max_byte_len: u64 = 5242880;
// const log_file_max_byte_len: u64 = 10240;

impl CFile {
    fn now(&self) -> String {
        let dt = Local::now();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    fn nowDate(&self) -> String {
    	let dt = Local::now();
    	dt.format("%Y-%m-%d").to_string()
    }

    fn deleteDirs(&mut self, path: &str, now: &DateTime<Local>) {
        /*
        ** 计算出应该删除的日期的最小值
        */
        let naiveLocal = now.naive_local();
        let nowTimestamp = naiveLocal.timestamp();
        let minTimestamp = nowTimestamp - self.maxSecond;
        let minNaiveDateTime = NaiveDateTime::from_timestamp(minTimestamp, 0);
        let minNaiveDate = minNaiveDateTime.date();
        let dirs = match Path::new(path).read_dir() {
            Ok(ds) => ds,
            Err(err) => {
                println!("read dir error, err: {}", err);
                return;
            }
        };
        for dir in dirs {
            let dir = match dir {
                Ok(d) => d,
                Err(err) => {
                    continue;
                }
            };
            let pathBuf = dir.path();
            if !pathBuf.is_dir() {
                continue;
            }
            let name = match dir.file_name().to_str() {
                Some(n) => n.to_string(),
                None => {
                    continue;
                }
            };
            let dirDate = match NaiveDate::parse_from_str(&name, "%Y-%m-%d") {
                Ok(dt) => dt,
                Err(err) => {
                    println!("parse from str error, err: {}", err);
                    continue;
                }
            };
            if dirDate >= minNaiveDate {
                continue;
            }
            if let Err(err) = fs::remove_dir_all(Path::new(path).join(name.clone())) {
                println!("remove_dir_all error, err: {}", err);
            } else {
                println!("remove {:?} success", Path::new(path).join(name.clone()));
            }
        }
    }

    fn createDir(&mut self, path: &str, contentType: &str) -> Result<PathBuf, &str> {
        let nowDatetime = Local::now();
        let nowDate = nowDatetime.date();
        match self.date {
            Some(d) => {
                /*
                ** 判断当前时间是否和d不一样
                **  不一样(说明处于时间切换点) => 检测是否需要删除目录, 并对 self.date 赋值
                **  一样 => 不处理
                */
                if (d != nowDate) {
                    self.deleteDirs(path, &nowDatetime);
                    self.date = Some(nowDate);
                }
            },
            None => {
                /*
                ** 检测是否需要删除目录, 并对 self.date 赋值
                */
                self.deleteDirs(path, &nowDatetime);
                self.date = Some(nowDate);
            }
        }
    	let date = self.nowDate();
    	let full = Path::new(path).join(date).as_path().join(contentType);
    	if full.as_path().exists() {
    		return Ok(full);
    	}
    	if let Ok(_) = DirBuilder::new().recursive(true).create(&full) {
    		return Ok(full);
    	}
    	Err("create dirs error")
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

    fn findFile1<'a>(&self, dir: &'a str) -> Result<PathBuf, &str> {
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

    fn joinPath(&self, dir: &str, index: u64) -> String {
        let mut path = String::new();
        path.push_str(dir);
        path.push_str("/");
        path.push_str(&index.to_string());
        path.push_str(".log");
        path
    }

    fn findFile<'a>(&self, dir: &'a str) -> Result<String, &str> {
        let mut index: u64 = 1;
        loop {
            let path = self.joinPath(dir, index);
            if Path::new(&path).exists() {
                let meta = match fs::metadata(&path) {
                    Ok(m) => m,
                    Err(err) => {
                        return Err("get metadata error");
                    }
                };
                if meta.len() > log_file_max_byte_len {
                    index += 1;
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
            index += 1;
        }
        Ok(self.joinPath(dir, index))
    }

    fn w(&mut self, root: &str, contentType: &str, content: &str) -> std::io::Result<()> {
    	if let Ok(dirOsString) = self.createDir(root, contentType) {
    		if let Ok(rootDir) = dirOsString.into_os_string().into_string() {
		    	if let Ok(path) = self.findFile(rootDir.as_str()) {
			        let f = OpenOptions::new().append(true).create(true).open(path)?;
			        let mut writer = BufWriter::new(f);
			        writer.write(content.as_bytes())?;
			        writer.flush()?;
			    }
			}
		}
        Ok(())
    }
}

impl IStorage for CFile {
    fn write(&mut self, path: &str, logType: &str, content: &str) -> std::io::Result<()> {
        self.w(path, logType, content)
    }
}

impl CFile {
    pub fn new(maxDay: i64) -> CFile {
        CFile{
            maxSecond: maxDay * 3600 * 24,
            date: None
        }
    }
}

