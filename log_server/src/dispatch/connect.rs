// extern crate serde_json;
extern crate rustc_serialize;

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
use std::io::prelude::*;

use rustc_serialize::json;

use super::super::pool::thread::CThreadPool;
use super::super::storage::IStorage;
use super::super::storage::file::CFile;

const requestModeConnect: &str = "connect";
const requestModeSending: &str = "sending";
const requestIdentifyPublish: &str = "publish";
const requestIdentifySubscribe: &str = "subscribe";

// #[derive(Serialize, Deserialize)]
#[derive(RustcDecodable, RustcEncodable)]
pub struct CRequest {
    mode: String,
    identify: String,
    serverName: String,
    serverVersion: String,
    serverNo: String,
    topic: String,
    data: String
}

pub struct CSubscribeInfo {
    stream: TcpStream,
    topic: String
}

pub struct CConnect {
    subscribes: HashMap<String, Vec<CSubscribeInfo>>,
    queuePool: CThreadPool,
    storageFile: CFile
}

impl CConnect {
    fn joinKey(serverName: String, serverVersion: String, serverNo: String) -> String {
        let key = vec![serverName, serverVersion, serverNo].join("-");
        key
    }

    pub fn start(self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        let subscribes = Arc::new(Mutex::new(self.subscribes));
        let threadPool = Arc::new(Mutex::new(self.queuePool));
        let storageFile = Arc::new(Mutex::new(self.storageFile));
        for stream in listener.incoming() {
            let subscribes = subscribes.clone();
            let threadPool = threadPool.clone();
            let storageFile = storageFile.clone();
            thread::spawn(move || {
                let stream = stream.unwrap();
                let reader = BufReader::new(&stream);
                for line in reader.lines() {
                    let line = line.unwrap();
                    // let request: CRequest = serde_json::from_str(&line).unwrap();
                    let request: CRequest = json::decode(&line).unwrap();
                    if request.mode == requestModeConnect && request.identify == requestIdentifyPublish {
                        let key = CConnect::joinKey(request.serverName, request.serverVersion, request.serverNo);
                        // create subscribes map
                        let mut subs = subscribes.lock().unwrap();
                        subs.insert(key, Vec::new());
                    } else if request.mode == requestModeSending && request.identify == requestIdentifyPublish {
                        // handle server send data
                        let key = CConnect::joinKey(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone());
                        // broadcast in thread pool
                        let pool = threadPool.lock().unwrap();
                        let subscribes = subscribes.clone();
                        let storageFile = storageFile.clone();
                        pool.execute(move || {
                            let mut subs = subscribes.lock().unwrap();
                            let mut sf = storageFile.lock().unwrap();
                            if let Some(subQueue) = subs.get_mut(&key) {
                                let mut removes = Vec::new();
                                let mut index = 0;
                                for sub in &(*subQueue) {
                                    let mut writer = BufWriter::new(&sub.stream);
                                    let content = vec![request.data.clone(), "\n".to_string()].join("");
                                    writer.write_all(content.as_bytes());
                                    if let Err(e) = writer.flush() {
                                        // (*subQueue).remove_item(sub);
                                        removes.push(index);
                                    } else {
                                        sf.message(&key, &content);
                                    }
                                    index += 1;
                                }
                                for removeIndex in removes {
                                    (*subQueue).remove(removeIndex);
                                }
                            };
                        });
                    } else if request.mode == requestModeConnect && request.identify == requestIdentifySubscribe {
                        let key = CConnect::joinKey(request.serverName, request.serverVersion, request.serverNo);
                        let mut subs = subscribes.lock().unwrap();
                        match subs.get_mut(&key) {
                            Some(value) => {
                                let sub = CSubscribeInfo {
                                    stream: stream,
                                    topic: request.topic
                                };
                                (*value).push(sub);
                            },
                            None => break
                        };
                        break;
                    }
                }
            });
        }
    }
}

impl CConnect {
    pub fn new(queueThreadMax: usize) -> CConnect {
        let conn = CConnect{
            subscribes: HashMap::new(),
            queuePool: CThreadPool::new(queueThreadMax),
            storageFile: CFile::new()
        };
        conn
    }
}
