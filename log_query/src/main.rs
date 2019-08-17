extern crate rust_parse;
extern crate log_query;

use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::net::TcpStream;
use std::{thread, time};

use rust_parse::cmd::CCmd;

use log_query::decode::stream::CStreamBlockParse;

const requestModeConnect: &str = "connect";
const requestModeSending: &str = "sending";
const requestModeQuery: &str = "query";
const requestIdentifyPublish: &str = "publish";
const requestIdentifySubscribe: &str = "subscribe";
const requestIdentifyQueryer: &str = "queryer";
const storageModeNone: &str = "none";
const storageModeFile: &str = "file";
const logTypeMessage: &str = "message";
const logTypeError: &str = "error";

const argHelp: &str = "-help";
const argServer: &str = "-server";
const argServerName: &str = "-server-name";
const argServerVersion: &str = "-server-version";
const argServerNo: &str = "-server-no";
const argStorageMode: &str = "-storage-mode";
const argLogType: &str = "-log-type";
const argTopic: &str = "-topic";
const argConnectCheck: &str = "-connect-check";
const argKeyword: &str = "-keyword";

struct CRequest {
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

#[derive(Default)]
struct CResponse {
    mode: String,
    keyword: String,
    data: String
}

macro_rules! decode_response {
    ($index:ident, $s:ident, $res:ident) => ({
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
        if $index == 1 {$res.mode = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 3 {$res.keyword = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        else if $index == 5 {$res.data = match String::from_utf8($s) {
            Ok(v) => v,
            Err(_) => "".to_string()
        }}
        if $index == 5 {
            return (false, 0);
        }
        return (true, 32);
    })
}

fn append32Number(value: u32, buf: &mut Vec<u8>) {
    for i in 0..32 {
        let b = (value >> i) & 1;
        buf.push(b as u8);
    }
}

fn sendRequest(request: CRequest, stream: TcpStream) -> bool {
    let mut writer = BufWriter::new(&stream);
    let mut buf = Vec::new();
    append32Number(request.mode.len() as u32, &mut buf);
    buf.append(&mut request.mode.as_bytes().to_vec());
    append32Number(request.identify.len() as u32, &mut buf);
    buf.append(&mut request.identify.as_bytes().to_vec());
    append32Number(request.serverName.len() as u32, &mut buf);
    buf.append(&mut request.serverName.as_bytes().to_vec());
    append32Number(request.serverVersion.len() as u32, &mut buf);
    buf.append(&mut request.serverVersion.as_bytes().to_vec());
    append32Number(request.serverNo.len() as u32, &mut buf);
    buf.append(&mut request.serverNo.as_bytes().to_vec());
    append32Number(request.topic.len() as u32, &mut buf);
    buf.append(&mut request.topic.as_bytes().to_vec());
    append32Number(request.data.len() as u32, &mut buf);
    buf.append(&mut request.data.as_bytes().to_vec());
    append32Number(request.storageMode.len() as u32, &mut buf);
    buf.append(&mut request.storageMode.as_bytes().to_vec());
    append32Number(request.logType.len() as u32, &mut buf);
    buf.append(&mut request.logType.as_bytes().to_vec());
    append32Number(request.keyword.len() as u32, &mut buf);
    buf.append(&mut request.keyword.as_bytes().to_vec());
    if let Err(err) = writer.write_all(&buf) {
        return false;
    };
    if let Err(err) = writer.flush() {
        return false;
    };
    true
}

fn main() {
    let mut cmdHandler = CCmd::new();
    let help = cmdHandler.register(argHelp, "");
    let server = cmdHandler.register(argServer, "127.0.0.1:50005");
    let serverName = cmdHandler.register(argServerName, "tests");
    let serverVersion = cmdHandler.register(argServerVersion, "1.0");
    let serverNo = cmdHandler.register(argServerNo, "1");
    let topic = cmdHandler.register(argTopic, "");
    let logType = cmdHandler.register(argLogType, "");
    let connectCheck = cmdHandler.register(argConnectCheck, "1000");
    let keyword = cmdHandler.register(argKeyword, "");
    cmdHandler.parse();

    let help = help.borrow();
    let server = server.borrow();
    let serverName = serverName.borrow();
    let serverVersion = serverVersion.borrow();
    let serverNo = serverNo.borrow();
    let topic = topic.borrow();
    let logType = logType.borrow();
    let connectCheck = connectCheck.borrow();
    let keyword = keyword.borrow();

    let connectCheck = connectCheck.parse::<u64>().unwrap();

    // is exists help
    if *help == "doc" {
        let mut message = String::new();
        message.push_str("options: \n");
        message.push_str("\t-server: log server addr, exp: localhost:50005\n");
        message.push_str("\t-server-name: server name, exp: cfgs\n");
        message.push_str("\t-server-version: server version, exp: 1.0\n");
        message.push_str("\t-server-no: server no, exp: 1\n");
        message.push_str("\t-storage-mode: storage mode, exp: file, mysql (support file current)\n");
        message.push_str("\t-log-type: log type, exp: \n");
        message.push_str("\t-keyword: memory query keyword, exp: \n");
        message.push_str("\t\tmessage <==> println\n");
        message.push_str("\t\tdebug <==> debugln\n");
        message.push_str("\t\tinfo <==> infoln\n");
        message.push_str("\t\twarn <==> warnln\n");
        message.push_str("\t\terror <==> errorln\n");
        message.push_str("\t\tfatal <==> fatalln\n");
        message.push_str("\t-topic: topic, exp: /get\n");
        print!("{}", message);
    } else {
        // not exists help

        loop {
            if let Ok(stream) = TcpStream::connect(&(*server)) {
                let stream = TcpStream::connect(&(*server)).unwrap();
                let mut reader = BufReader::new(&stream);
                let mut writer = BufWriter::new(&stream);

                let connRequest = CRequest {
                    mode: requestModeConnect.to_string(),
                    identify: requestIdentifyQueryer.to_string(),
                    serverName: serverName.to_string(),
                    serverVersion: serverVersion.to_string(),
                    serverNo: serverNo.to_string(),
                    topic: "".to_string(),
                    data: "".to_string(),
                    storageMode: "".to_string(),
                    logType: "".to_string(),
                    keyword: keyword.to_string()
                };
                sendRequest(connRequest, stream.try_clone().unwrap());
                /*
                let encoded = json::encode(&connRequest).unwrap();
                let content = vec![encoded, "\n".to_string()].join("");
                writer.write_all(content.as_bytes()).unwrap();
                writer.flush().unwrap();
                */

                let mut res = CResponse::default();
                let mut r = CStreamBlockParse::new(stream.try_clone().unwrap());
                r.lines(32, &mut res, &mut |index: u64, buf: Vec<u8>, response: &mut CResponse| -> (bool, u32) {
                    decode_response!(index, buf, response);
                }, |response: &CResponse| -> bool {
                    print!("***keyword: \n{}\n", response.keyword);
                    print!("***memory info: \n{}\n", response.data);
                    return true;
                });

                /*
                let mut buffer = String::new();
                while let Ok(size) = reader.read_line(&mut buffer) {
                    if size > 0 {
                        if buffer != "\n".to_string() {
                            print!("{}", buffer);
                        }
                        buffer.clear();
                    }
                }
                */
                println!("closed");
            } else {
                thread::sleep(time::Duration::from_millis(connectCheck));
            }
        }
    }
}
