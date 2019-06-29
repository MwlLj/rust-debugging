// extern crate serde_json;
extern crate rustc_serialize;

use std::error::Error;
use std::collections::HashMap;
use std::io;
use std::net::TcpListener;
use std::net::TcpStream;
use std::io::BufReader;
use std::io::BufWriter;
use std::thread;
use std::sync::mpsc;
use std::sync::Mutex;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;

use rustc_serialize::json;

use super::super::pool::thread::CThreadPool;
use super::super::storage::IStorage;
use super::super::storage::file::CFile;
use super::super::statics::content::CContent;
use super::super::decode::stream::CStreamBlockParse;

const requestModeConnect: &str = "connect";
const requestModeSending: &str = "sending";
const requestIdentifyPublish: &str = "publish";
const requestIdentifySubscribe: &str = "subscribe";
const storageModeNone: &str = "none";
const storageModeFile: &str = "file";
const logTypeMessage: &str = "message";
const logTypeError: &str = "error";

// #[derive(Serialize, Deserialize)]
#[derive(RustcDecodable, RustcEncodable, Default)]
pub struct CRequest {
    mode: String,
    identify: String,
    serverName: String,
    serverVersion: String,
    serverNo: String,
    topic: String,
    data: String,
    storageMode: String,
    logType: String
}

macro_rules! decode_request {
    ($index:ident, $s:ident, $req:ident) => ({
        if $index % 2 == 0 {
            let two: u32 = 2;
            let mut number = 0;
            let mut i = 0;
            for item in $s {
                // println!("{}, {}, {}", item, i, two.pow(i));
                number += item as u32 * two.pow(i);
                i += 1;
            }
            return (true, number);
        }
        if $index == 1 {$req.mode = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 3 {$req.identify = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 5 {$req.serverName = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 7 {$req.serverVersion = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 9 {$req.serverNo = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 11 {$req.topic = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 13 {$req.data = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 15 {$req.storageMode = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 17 {$req.logType = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        if $index == 17 {
            return (false, 0);
        }
        return (true, 32);
    })
}

pub struct CSubscribeInfo {
    stream: TcpStream,
    topic: String,
    logType: String,
    serverName: String,
    serverVersion: String,
    serverNo: String
}

pub struct CConnect {
    subscribes: HashMap<String, Vec<CSubscribeInfo>>,
    queuePool: CThreadPool,
    storageFile: CFile,
    content: CContent
}

impl CConnect {
    fn joinKey(serverName: String, serverVersion: String, serverNo: String) -> String {
    	if serverName != "" && serverVersion == "" && serverNo == "" {
    		// to do
    	}
        let key = vec![serverName, serverVersion, serverNo].join("-");
        key
    }

    pub fn start(self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        let subscribes = Arc::new(Mutex::new(self.subscribes));
        let threadPool = Arc::new(Mutex::new(self.queuePool));
        let storageFile = Arc::new(Mutex::new(self.storageFile));
        let contentStatic = Arc::new(Mutex::new(self.content));
        for stream in listener.incoming() {
            let subscribes = subscribes.clone();
            let threadPool = threadPool.clone();
            let storageFile = storageFile.clone();
            let contentStatic = contentStatic.clone();
            thread::spawn(move || {
                if let Ok(stream) = stream {
                    let mut reader = BufReader::new(&stream);
                    // loop {
                    //     let mut buf = vec![];
                    //     if let Err(_) = reader.read_until(b'\n', &mut buf) {
                    //         break;
                    //     }
                        // let body = String::from_utf8(buf);
                        // if let Ok(request) = json::decode(body.unwrap().as_str()) {
                    let mut req = CRequest::default();
                    let mut r = CStreamBlockParse::new(stream.try_clone().unwrap());
                    r.lines(32, &mut req, &mut |index: u64, buf: Vec<u8>, request: &mut CRequest| -> (bool, u32) {
                        decode_request!(index, buf, request);
                    }, |request: &CRequest| -> bool {
                        if request.mode == requestModeConnect && request.identify == requestIdentifyPublish {
                            let key = CConnect::joinKey(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone());
                            // create subscribes map
                            let mut subs = subscribes.lock().unwrap();
                            match subs.get_mut(&key) {
                                None => {
                                    subs.insert(key, Vec::new());
                                },
                                Some(_) => {}
                            }
                        } else if request.mode == requestModeSending && request.identify == requestIdentifyPublish {
                            CContent::handlePublish(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone()
                                , request.data.clone(), request.logType.clone(), request.storageMode.clone(), request.topic.clone()
                                , threadPool.clone(), subscribes.clone(), storageFile.clone(), contentStatic.clone());
                        } else if request.mode == requestModeConnect && request.identify == requestIdentifySubscribe {
                            let key = CConnect::joinKey(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone());
                            let mut subs = subscribes.lock().unwrap();
                            let sub = CSubscribeInfo {
                                stream: stream.try_clone().unwrap(),
                                topic: request.topic.clone(),
                                logType: request.logType.clone(),
                                serverName: request.serverName.clone(),
                                serverVersion: request.serverVersion.clone(),
                                serverNo: request.serverNo.clone()
                            };
                            match subs.get_mut(&key) {
                                Some(value) => {
                                    (*value).push(sub);
                                },
                                None => {
                                    let mut v = Vec::new();
                                    v.push(sub);
                                    subs.insert(key, v);
                                }
                            };
                            return false;
                        }
                        return true;
                    });
                    /*
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            if let Ok(request) = json::decode(&line) {
                                let request: CRequest = request;
                                if request.mode == requestModeConnect && request.identify == requestIdentifyPublish {
                                    let key = CConnect::joinKey(request.serverName, request.serverVersion, request.serverNo);
                                    // create subscribes map
                                    let mut subs = subscribes.lock().unwrap();
                                    match subs.get_mut(&key) {
                                        None => {
                                            subs.insert(key, Vec::new());
                                        },
                                        Some(_) => {}
                                    }
                                } else if request.mode == requestModeSending && request.identify == requestIdentifyPublish {
                                    // handle server send data
                                    let key = CConnect::joinKey(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone());
                                    // broadcast in thread pool
                                    let pool = threadPool.lock().unwrap();
                                    let subscribes = subscribes.clone();
                                    let storageFile = storageFile.clone();
                                    let contentStatic = contentStatic.clone();
                                    pool.execute(move || {
                                        let mut subs = subscribes.lock().unwrap();
                                        let sf = storageFile.lock().unwrap();
                                        let cs = contentStatic.lock().unwrap();
                                        if let Some(subQueue) = subs.get_mut(&key) {
                                            let mut removes = Vec::new();
                                            let mut index = 0;
                                            let content = vec![request.data.clone(), "\n".to_string()].join("");
                                            let content = cs.full(&key, &request.logType, &request.topic, &content);
                                            if request.storageMode == storageModeFile {
                                                sf.write(&key, &request.logType, &content);
                                                sf.write(&key, "full", &content);
                                                // if cfg!(all(target_os="linux", target_arch="arm")) {
                                                // } else {
                                                //     sf.write(&key, &request.logType, &content);
                                                // }
                                            }
                                            for sub in &(*subQueue) {
                                            	let mut isSend = false;
                                            	if (sub.topic != "" && sub.logType == "") && request.topic != sub.topic {
                                            		isSend = false;
                                            	} else if (sub.logType != "" && sub.topic == "") && request.logType != sub.logType {
                                            		isSend = false;
                                            	} else if (sub.logType != "" && sub.topic != "") && (request.logType != sub.logType || request.topic != sub.topic) {
                                            		isSend = false;
                                            	} else {
                                            		isSend = true;
            	                                }
                                                let mut writer = BufWriter::new(&sub.stream);
                                                if isSend {
            	                                    writer.write_all(content.as_bytes());
            	                                } else {
            	                                	writer.write_all(b"\n");
            	                                }
                                                if let Err(e) = writer.flush() {
                                                    // (*subQueue).remove_item(sub);
                                                    removes.push(index);
                                                }
                                                index += 1;
                                            }
                                            for removeIndex in removes {
                                                println!("remove index: {}", removeIndex);
                                                (*subQueue).remove(removeIndex);
                                            }
                                        };
                                    });
                                } else if request.mode == requestModeConnect && request.identify == requestIdentifySubscribe {
                                    let key = CConnect::joinKey(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone());
                                    let mut subs = subscribes.lock().unwrap();
                                    let sub = CSubscribeInfo {
                                        stream: stream,
                                        topic: request.topic,
                                        logType: request.logType,
                                        serverName: request.serverName,
                                        serverVersion: request.serverVersion,
                                        serverNo: request.serverNo
                                    };
                                    match subs.get_mut(&key) {
                                        Some(value) => {
                                            (*value).push(sub);
                                        },
                                        None => {
                                            let mut v = Vec::new();
                                            v.push(sub);
                                            subs.insert(key, v);
                                        }
                                    };
                                    break;
                                }
                            }
                        }
                    }
                    */
                }
            });
        }
    }
}

impl CContent {
    fn handlePublish(serverName: String, serverVersion: String, serverNo: String
        , data: String, logType: String, storageMode: String, topic: String
        , threadPool: Arc<Mutex<CThreadPool>>, subscribes: Arc<Mutex<HashMap<String, Vec<CSubscribeInfo>>>>
        , storageFile: Arc<Mutex<CFile>>, contentStatic: Arc<Mutex<CContent>>) {
        // handle server send data
        let key = CConnect::joinKey(serverName, serverVersion, serverNo);
        // broadcast in thread pool
        let pool = threadPool.lock().unwrap();
        let subscribes = subscribes.clone();
        let storageFile = storageFile.clone();
        let contentStatic = contentStatic.clone();
        pool.execute(move || {
            let mut subs = subscribes.lock().unwrap();
            let sf = storageFile.lock().unwrap();
            let cs = contentStatic.lock().unwrap();
            if let Some(subQueue) = subs.get_mut(&key) {
                let mut removes = Vec::new();
                let mut index = 0;
                let content = vec![data, "\n".to_string()].join("");
                let content = cs.full(&key, &logType, &topic, &content);
                if storageMode == storageModeFile {
                    sf.write(&key, &logType, &content);
                    sf.write(&key, "full", &content);
                    // if cfg!(all(target_os="linux", target_arch="arm")) {
                    // } else {
                    //     sf.write(&key, &request.logType, &content);
                    // }
                }
                for sub in &(*subQueue) {
                    let mut isSend = false;
                    if (sub.topic != "" && sub.logType == "") && topic != sub.topic {
                        isSend = false;
                    } else if (sub.logType != "" && sub.topic == "") && logType != sub.logType {
                        isSend = false;
                    } else if (sub.logType != "" && sub.topic != "") && (logType != sub.logType || topic != sub.topic) {
                        isSend = false;
                    } else {
                        isSend = true;
                    }
                    let mut res: Result<(), &str> = Ok(());
                    if isSend {
                        // writer.write_all(content.as_bytes());
                        res = CContent::sendContent(&content, sub.stream.try_clone().unwrap());
                    } else {
                        // writer.write_all(b"\n");
                        res = CContent::sendContent("\n", sub.stream.try_clone().unwrap());
                    }
                    if let Err(e) = res {
                        // (*subQueue).remove_item(sub);
                        removes.push(index);
                    }
                    // if let Err(e) = writer.flush() {
                    //     // (*subQueue).remove_item(sub);
                    //     removes.push(index);
                    // }
                    index += 1;
                }
                for removeIndex in removes {
                    println!("remove index: {}", removeIndex);
                    (*subQueue).remove(removeIndex);
                }
            };
        });
    }

    fn append32Number(value: u32, buf: &mut Vec<u8>) {
        for i in 0..32 {
            let b = (value >> i) & 1;
            buf.push(b as u8);
        }
    }

    fn sendContent(content: &str, stream: TcpStream) -> Result<(), &str> {
        let mut writer = BufWriter::new(&stream);
        let mut buf = Vec::new();
        CContent::append32Number(content.len() as u32, &mut buf);
        buf.append(&mut content.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return Err("write error");
        };
        if let Err(err) = writer.flush() {
            return Err("flush error");
        };
        Ok(())
    }

    fn sendRequest(request: CRequest, stream: TcpStream) -> bool {
        let mut writer = BufWriter::new(&stream);
        let mut buf = Vec::new();
        CContent::append32Number(request.mode.len() as u32, &mut buf);
        buf.append(&mut request.mode.as_bytes().to_vec());
        CContent::append32Number(request.identify.len() as u32, &mut buf);
        buf.append(&mut request.identify.as_bytes().to_vec());
        CContent::append32Number(request.serverName.len() as u32, &mut buf);
        buf.append(&mut request.serverName.as_bytes().to_vec());
        CContent::append32Number(request.serverVersion.len() as u32, &mut buf);
        buf.append(&mut request.serverVersion.as_bytes().to_vec());
        CContent::append32Number(request.serverNo.len() as u32, &mut buf);
        buf.append(&mut request.serverNo.as_bytes().to_vec());
        CContent::append32Number(request.topic.len() as u32, &mut buf);
        buf.append(&mut request.topic.as_bytes().to_vec());
        CContent::append32Number(request.data.len() as u32, &mut buf);
        buf.append(&mut request.data.as_bytes().to_vec());
        CContent::append32Number(request.storageMode.len() as u32, &mut buf);
        buf.append(&mut request.storageMode.as_bytes().to_vec());
        CContent::append32Number(request.logType.len() as u32, &mut buf);
        buf.append(&mut request.logType.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return false;
        };
        if let Err(err) = writer.flush() {
            return false;
        };
        true
    }
}

impl CConnect {
    pub fn new(queueThreadMax: usize) -> CConnect {
        let conn = CConnect{
            subscribes: HashMap::new(),
            queuePool: CThreadPool::new(queueThreadMax),
            storageFile: CFile::new(),
            content: CContent::new()
        };
        conn
    }
}

