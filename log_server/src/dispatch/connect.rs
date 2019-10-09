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
use std::time;

use rustc_serialize::json;

use super::super::pool::thread::CThreadPool;
use super::super::storage::IStorage;
use super::super::storage::file::CFile;
use super::super::statics::content::CContent;
use rust_parse::stream::tcp_block_mut::CStreamBlockParse;

const requestModeConnect: &str = "connect";
const requestModeSending: &str = "sending";
const requestModeQuery: &str = "query";
const requestModeQueryAck: &str = "query-ack";
const requestIdentifyPublish: &str = "publish";
const requestIdentifySubscribe: &str = "subscribe";
const requestIdentifyQueryer: &str = "queryer";
const storageModeNone: &str = "none";
const storageModeFile: &str = "file";
const logTypeMessage: &str = "message";
const logTypeError: &str = "error";
const askModeMemoryQuery: &str = "memory-query";
const responseModeQueryResult: &str = "query-result";

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
    logType: String,
    keyword: String
}

pub struct CResponse {
    mode: String,
    keyword: String,
    data: String
}

macro_rules! decode_request {
    ($index:ident, $s:ident, $req:ident) => ({
        if $index % 2 == 0 {
            let two: u64 = 2;
            let mut number = 0;
            let mut i = 0;
            for item in $s {
                // println!("{}, {}, {}", item, i, two.pow(i));
                number += item as u64 * two.pow(i);
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
        else if $index == 19 {$req.keyword = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        if $index == 19 {
            return (false, 0);
        }
        return (true, 32);
    })
}

trait IRemove {
    fn id(&self) -> &str;
}

pub struct CSubscribeInfo {
    connId: String,
    subKey: String,
    stream: TcpStream,
    topic: String,
    logType: String,
    serverName: String,
    serverVersion: String,
    serverNo: String
}

impl IRemove for CSubscribeInfo {
    fn id(&self) -> &str {
        &self.connId
    }
}

pub struct CQueryAck {
    keyword: String,
    data: String
}

pub struct CPublishInfo {
    connId: String,
    stream: TcpStream
}

impl IRemove for CPublishInfo {
    fn id(&self) -> &str {
        &self.connId
    }
}

pub struct CQueryInfo {
    connId: String,
    stream: TcpStream
}

impl IRemove for CQueryInfo {
    fn id(&self) -> &str {
        &self.connId
    }
}

pub struct CConnect {
    subscribes: HashMap<String, Vec<CSubscribeInfo>>,
    publishs: HashMap<String, Vec<CPublishInfo>>,
    queryers: HashMap<String, Vec<CQueryInfo>>,
    queuePool: CThreadPool,
    storageFile: CFile,
    content: CContent
}

impl CConnect {
    fn joinKey(serverName: &str, serverVersion: &str, serverNo: &str) -> String {
    	if serverName != "" && serverVersion == "" && serverNo == "" {
    		// to do
    	}
        let mut key = String::new();
        key.push_str(serverName);
        key.push_str("-");
        key.push_str(serverVersion);
        key.push_str("-");
        key.push_str(serverNo);
        key
    }

    pub fn start(self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        let subscribes = Arc::new(Mutex::new(self.subscribes));
        let publishs = Arc::new(Mutex::new(self.publishs));
        let queryers = Arc::new(Mutex::new(self.queryers));
        let threadPool = Arc::new(Mutex::new(self.queuePool));
        let storageFile = Arc::new(Mutex::new(self.storageFile));
        let contentStatic = Arc::new(Mutex::new(self.content));
        for stream in listener.incoming() {
            let subscribes = subscribes.clone();
            let publishs = publishs.clone();
            let queryers = queryers.clone();
            let threadPool = threadPool.clone();
            let storageFile = storageFile.clone();
            let contentStatic = contentStatic.clone();
            thread::spawn(move || {
                let stream = match stream {
                    Ok(s) => s,
                    Err(err) => {
                        println!("stream error, err: {}", err);
                        return;
                    }
                };
                let connId = uuid::Uuid::new_v4().to_string();
                let reader = BufReader::new(&stream);
                let mut req = CRequest::default();
                let mut r = CStreamBlockParse::new(stream.try_clone().unwrap());
                #[derive(Default)]
                struct PCTmp(String, String, String);
                let mut pubCon = PCTmp::default();
                r.lines(32, &mut req, &mut |index: u64, buf: Vec<u8>, request: &mut CRequest| -> (bool, u64) {
                    decode_request!(index, buf, request);
                }, &mut pubCon, &mut |request: &mut CRequest, pc: &mut PCTmp| -> bool {
                    if request.mode == requestModeConnect && request.identify == requestIdentifyPublish {
                        let mut publisherKey = CConnect::joinKey(&request.serverName, &request.serverVersion, &request.serverNo);
                        let mut pubs = publishs.lock().unwrap();
                        let pu = CPublishInfo {
                            connId: connId.clone(),
                            stream: stream.try_clone().unwrap()
                        };
                        match pubs.get_mut(&publisherKey) {
                            Some(value) => {
                                (*value).push(pu);
                            },
                            None => {
                                let mut v = Vec::new();
                                v.push(pu);
                                pubs.insert(publisherKey.clone(), v);
                            }
                        };
                        /*
                        // create subscribes map
                        let mut subs = subscribes.lock().unwrap();
                        match subs.get_mut(&publisherKey) {
                            None => {
                                subs.insert(publisherKey.clone(), Vec::new());
                            },
                            Some(_) => {}
                        }
                        */
                        pc.0 = publisherKey;
                    } else if request.mode == requestModeSending && request.identify == requestIdentifyPublish {
                        CConnect::handlePublish(request.serverName.clone(), request.serverVersion.clone(), request.serverNo.clone()
                            , request.data.clone(), request.logType.clone(), request.storageMode.clone(), request.topic.clone()
                            , threadPool.clone(), subscribes.clone(), storageFile.clone(), contentStatic.clone());
                    } else if request.mode == requestModeQueryAck && request.identify == requestIdentifyPublish {
                        CConnect::handleQueryAck(&request.serverName, &request.serverVersion
                            , &request.serverNo, &request.keyword, &request.data
                            , queryers.clone());
                    } else if request.mode == requestModeQuery && request.identify == requestIdentifyQueryer {
                        println!("handle query");
                        CConnect::handleQuery(request.serverName.clone(), request.serverVersion.clone()
                            , request.serverNo.clone(), request.keyword.clone(), publishs.clone());
                    } else if request.mode == requestModeConnect && request.identify == requestIdentifySubscribe {
                        let mut consumerKey = CConnect::joinKey(&request.serverName, &request.serverVersion, &request.serverNo);
                        let mut subs = subscribes.lock().unwrap();
                        let sub = CSubscribeInfo {
                            connId: connId.clone(),
                            subKey: consumerKey.clone(),
                            stream: stream.try_clone().unwrap(),
                            topic: request.topic.clone(),
                            logType: request.logType.clone(),
                            serverName: request.serverName.clone(),
                            serverVersion: request.serverVersion.clone(),
                            serverNo: request.serverNo.clone()
                        };
                        /*
                        let mut isFind = false;
                        for (key, value) in subs.iter_mut() {
                            if key.starts_with(&consumerKey) {
                                (*value).push(sub);
                                isFind = true;
                                break;
                            }
                        }
                        */
                        /*
                        ** When the sub runs first, check if the container exists.
                        ** If it exists, insert it into vec
                        ** If it does not exist, create vec
                        */
                        /*
                        if !isFind {
                            let mut v = Vec::new();
                            v.push(sub);
                            subs.insert(consumerKey.clone(), v);
                        }
                        */
                        match subs.get_mut(&consumerKey) {
                            Some(value) => {
                                (*value).push(sub);
                            },
                            None => {
                                let mut v = Vec::new();
                                v.push(sub);
                                subs.insert(consumerKey.clone(), v);
                            }
                        };
                        /*
                        */
                        pc.1 = consumerKey;
                    } else if request.mode == requestModeConnect && request.identify == requestIdentifyQueryer {
                        let mut queryerKey = CConnect::joinKey(&request.serverName, &request.serverVersion, &request.serverNo);
                        let mut queryers = queryers.lock().unwrap();
                        let queryer = CQueryInfo {
                            connId: connId.clone(),
                            stream: stream.try_clone().unwrap()
                        };
                        match queryers.get_mut(&queryerKey) {
                            Some(value) => {
                                (*value).push(queryer);
                            },
                            None => {
                                let mut v = Vec::new();
                                v.push(queryer);
                                queryers.insert(queryerKey.clone(), v);
                            }
                        };
                        pc.2 = queryerKey;
                    }
                    return true;
                });
                // publisher / consumer / queryer closed
                let publisherKey = pubCon.0;
                let consumerKey = pubCon.1;
                let queryerKey = pubCon.2;
                // println!("{}, {}, {}", &publisherKey, &consumerKey, &queryerKey);
                if publisherKey != "" {
                    let mut pubs = match publishs.lock() {
                        Ok(p) => p,
                        Err(err) => {
                            println!("publishs lock error, err: {}", err);
                            return;
                        }
                    };
                    pubs.remove(&publisherKey);
                    /*
                    let mut subs = match subscribes.lock() {
                        Ok(s) => s,
                        Err(err) => {
                            println!("subscribes lock error, err: {}", err);
                            return;
                        }
                    };
                    subs.remove(&publisherKey);
                    */
                } else if queryerKey != "" {
                    let mut queryers = match queryers.lock() {
                        Ok(p) => p,
                        Err(err) => {
                            println!("queryers lock error, err: {}", err);
                            return;
                        }
                    };
                    // queryers.remove(&queryerKey);
                    for (_, value) in queryers.iter_mut() {
                        for (i, item) in value.iter().enumerate() {
                            if item.connId == connId {
                                value.remove(i);
                                println!("after move, len: {}", value.len());
                                break;
                            }
                        }
                    }
                } else if consumerKey != "" {
                    let mut subs = match subscribes.lock() {
                        Ok(s) => s,
                        Err(err) => {
                            println!("subscribes lock error, err: {}", err);
                            return;
                        }
                    };
                    for (_, value) in subs.iter_mut() {
                        for (i, item) in value.iter().enumerate() {
                            if item.connId == connId {
                                value.remove(i);
                                println!("after move, len: {}", value.len());
                                break;
                            }
                        }
                    }
                    /*
                    let pubs = match publishs.lock() {
                        Ok(p) => p,
                        Err(err) => {
                            println!("publishs lock error, err: {}", err);
                            return;
                        }
                    };
                    for key in pubs.keys() {
                        let queue = match subs.get_mut(key) {
                            Some(v) => v,
                            None => {
                                println!("subscribe is not exist");
                                return;
                            }
                        };
                        let mut rmIds = Vec::new();
                        for item in queue.iter() {
                            if item.subKey == consumerKey {
                                rmIds.push(item.subKey.to_string());
                                break;
                            }
                        }
                        if rmIds.len() > 0 {
                            CConnect::removeItemsFromSet(&rmIds, queue);
                        }
                    }
                    */
                }
            });
        }
    }
}

impl CConnect {
    fn handlePublish(serverName: String, serverVersion: String, serverNo: String
        , data: String, logType: String, storageMode: String, topic: String
        , threadPool: Arc<Mutex<CThreadPool>>, subscribes: Arc<Mutex<HashMap<String, Vec<CSubscribeInfo>>>>
        , storageFile: Arc<Mutex<CFile>>, contentStatic: Arc<Mutex<CContent>>) {
        // handle server send data
        let key = CConnect::joinKey(&serverName, &serverVersion, &serverNo);
        // broadcast in thread pool
        let pool = threadPool.lock().unwrap();
        let subscribes = subscribes.clone();
        let storageFile = storageFile.clone();
        let contentStatic = contentStatic.clone();
        pool.execute(move || {
            /*
            ** write file
            */
            let sf = storageFile.lock().unwrap();
            let cs = contentStatic.lock().unwrap();
            let mut content = String::new();
            if cfg!(target_os="windows") {
                content.push_str(&data);
                content.push_str("\r\n");
            } else {
                content.push_str(&data);
                content.push_str("\n");
            }
            let content = cs.full(&key, &logType, &topic, &content);
            if storageMode == storageModeFile {
                sf.write(&key, &logType, &content);
                sf.write(&key, "full", &content);
                // if cfg!(all(target_os="linux", target_arch="arm")) {
                // } else {
                //     sf.write(&key, &request.logType, &content);
                // }
            }
            /*
            ** send to subscribes
            */
            let mut subs = subscribes.lock().unwrap();
            for (k, subQueue) in subs.iter_mut() {
                // println!("key: {}, k: {}", key, k);
                if !key.starts_with(k) {
                    continue;
                }
                let mut removes = Vec::new();
                for sub in &(*subQueue) {
                    // println!("send to subscribers");
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
                        res = CConnect::sendContent(&content, sub.stream.try_clone().unwrap());
                    } else {
                        // writer.write_all(b"\n");
                        res = CConnect::sendContent("\n", sub.stream.try_clone().unwrap());
                    }
                    if let Err(e) = res {
                        // (*subQueue).remove_item(sub);
                        removes.push(sub.connId.to_string());
                    }
                    // if let Err(e) = writer.flush() {
                    //     // (*subQueue).remove_item(sub);
                    //     removes.push(index);
                    // }
                }
                if removes.len() > 0 {
                    CConnect::removeItemsFromSet(&removes, &mut (*subQueue));
                }
            }
            /*
            if let Some(subQueue) = subs.get_mut(&key) {
            };
            */
        });
    }

    fn handleQuery(serverName: String, serverVersion: String, serverNo: String
        , keyword: String, publishs: Arc<Mutex<HashMap<String, Vec<CPublishInfo>>>>) {
        // handle server send data
        let key = CConnect::joinKey(&serverName, &serverVersion, &serverNo);
        // broadcast in thread pool
        let publishs = publishs.clone();
        thread::spawn(move || {
            let mut pubs = match publishs.lock() {
                Ok(ps) => ps,
                Err(err) => {
                    println!("publishs lock error, err: {}", err);
                    return;
                }
            };
            let pubQueue = match pubs.get_mut(&key) {
                Some(pq) => pq,
                None => {
                    println!("get pub error, key: {}", &key);
                    return;
                }
            };
            let mut removes = Vec::new();
            for pu in &(*pubQueue) {
                loop {
                    let res = CConnect::askQuery(&keyword, pu.stream.try_clone().unwrap());
                    if let Err(e) = res {
                        // removes.push(index);
                        removes.push(pu.connId.to_string());
                        break;
                    }
                    break;
                }
            }
            if removes.len() > 0 {
                CConnect::removeItemsFromSet(&removes, &mut (*pubQueue));
            }
            /*
            for removeIndex in removes {
                println!("remove index: {}", removeIndex);
                (*pubQueue).remove(removeIndex);
            }
            */
        });
    }

    fn handleQueryAck(serverName: &str, serverVersion: &str, serverNo: &str
        , keyword: &str, data: &str, queryers: Arc<Mutex<HashMap<String, Vec<CQueryInfo>>>>) {
        // handle server send data
        let key = CConnect::joinKey(serverName, serverVersion, serverNo);
        // broadcast in thread pool
        let queryers = queryers.clone();
        let keyword = keyword.to_string();
        let data = data.to_string();
        thread::spawn(move || {
            let mut queryers = match queryers.lock() {
                Ok(ps) => ps,
                Err(err) => {
                    println!("queryers lock error, err: {}", err);
                    return;
                }
            };
            let queryQueue = match queryers.get_mut(&key) {
                Some(pq) => pq,
                None => {
                    println!("get pub error, key: {}", &key);
                    return;
                }
            };
            let mut removes = Vec::new();
            for pu in &(*queryQueue) {
                loop {
                    let r = CConnect::sendResponse(CResponse{
                        mode: responseModeQueryResult.to_string(),
                        keyword: keyword.to_string(),
                        data: data.to_string()
                    }, pu.stream.try_clone().unwrap());
                    if !r {
                        // removes.push(index);
                        removes.push(pu.connId.to_string());
                        break;
                    }
                    break;
                }
            }
            if removes.len() > 0 {
                CConnect::removeItemsFromSet(&removes, &mut (*queryQueue));
            }
            /*
            for removeIndex in removes {
                println!("remove index: {}", removeIndex);
                (*queryQueue).remove(removeIndex);
            }
            */
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
        CConnect::append32Number(content.len() as u32, &mut buf);
        buf.append(&mut content.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return Err("write error");
        };
        if let Err(err) = writer.flush() {
            return Err("flush error");
        };
        Ok(())
    }

    fn askQuery(keyword: &str, stream: TcpStream) -> Result<(), &str> {
        let mut writer = BufWriter::new(&stream);
        let mut buf = Vec::new();
        CConnect::append32Number(askModeMemoryQuery.len() as u32, &mut buf);
        buf.append(&mut askModeMemoryQuery.as_bytes().to_vec());
        CConnect::append32Number(keyword.len() as u32, &mut buf);
        buf.append(&mut keyword.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return Err("write error");
        };
        if let Err(err) = writer.flush() {
            return Err("flush error");
        };
        Ok(())
    }

    fn sendResponse(response: CResponse, stream: TcpStream) -> bool {
        let mut writer = BufWriter::new(&stream);
        let mut buf = Vec::new();
        CConnect::append32Number(response.mode.len() as u32, &mut buf);
        buf.append(&mut response.mode.as_bytes().to_vec());
        CConnect::append32Number(response.keyword.len() as u32, &mut buf);
        buf.append(&mut response.keyword.as_bytes().to_vec());
        CConnect::append32Number(response.data.len() as u32, &mut buf);
        buf.append(&mut response.data.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return false;
        };
        if let Err(err) = writer.flush() {
            return false;
        };
        true
    }

    fn sendRequest(request: CRequest, stream: TcpStream) -> bool {
        let mut writer = BufWriter::new(&stream);
        let mut buf = Vec::new();
        CConnect::append32Number(request.mode.len() as u32, &mut buf);
        buf.append(&mut request.mode.as_bytes().to_vec());
        CConnect::append32Number(request.identify.len() as u32, &mut buf);
        buf.append(&mut request.identify.as_bytes().to_vec());
        CConnect::append32Number(request.serverName.len() as u32, &mut buf);
        buf.append(&mut request.serverName.as_bytes().to_vec());
        CConnect::append32Number(request.serverVersion.len() as u32, &mut buf);
        buf.append(&mut request.serverVersion.as_bytes().to_vec());
        CConnect::append32Number(request.serverNo.len() as u32, &mut buf);
        buf.append(&mut request.serverNo.as_bytes().to_vec());
        CConnect::append32Number(request.topic.len() as u32, &mut buf);
        buf.append(&mut request.topic.as_bytes().to_vec());
        CConnect::append32Number(request.data.len() as u32, &mut buf);
        buf.append(&mut request.data.as_bytes().to_vec());
        CConnect::append32Number(request.storageMode.len() as u32, &mut buf);
        buf.append(&mut request.storageMode.as_bytes().to_vec());
        CConnect::append32Number(request.logType.len() as u32, &mut buf);
        buf.append(&mut request.logType.as_bytes().to_vec());
        CConnect::append32Number(request.keyword.len() as u32, &mut buf);
        buf.append(&mut request.keyword.as_bytes().to_vec());
        if let Err(err) = writer.write_all(&buf) {
            return false;
        };
        if let Err(err) = writer.flush() {
            return false;
        };
        true
    }

    fn removeItemsFromSet<T: IRemove>(rmIds: &Vec<String>, queue: &mut Vec<T>) {
        for id in rmIds {
            match queue.iter().position(|x| {
                if &x.id() == id {
                    true
                } else {
                    false
                }
            }) {
                Some(pos) => {
                    println!("remove, pos: {}", pos);
                    queue.remove(pos);
                },
                None => {
                }
            }
        }
        println!("after remove, queue size: {}", queue.len());
    }
}

impl CConnect {
    pub fn new(queueThreadMax: usize) -> CConnect {
        let conn = CConnect{
            subscribes: HashMap::new(),
            publishs: HashMap::new(),
            queryers: HashMap::new(),
            queuePool: CThreadPool::new(queueThreadMax),
            storageFile: CFile::new(),
            content: CContent::new()
        };
        conn
    }
}

