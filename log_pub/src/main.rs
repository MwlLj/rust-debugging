extern crate rust_parse;
extern crate rustc_serialize;

use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::net::TcpStream;
use std::thread;

use rustc_serialize::json;
use rust_parse::cmd::CCmd;

const requestModeConnect: &str = "connect";
const requestModeSending: &str = "sending";
const requestIdentifyPublish: &str = "publish";
const requestIdentifySubscribe: &str = "subscribe";
const storageModeNone: &str = "none";
const storageModeFile: &str = "file";
const logTypeMessage: &str = "message";
const logTypeError: &str = "error";

const argServer: &str = "-server";
const argServerName: &str = "-server-name";
const argServerVersion: &str = "-server-version";
const argServerNo: &str = "-server-no";
const argData: &str = "-data";
const argStorageMode: &str = "-storage-mode";
const argLogType: &str = "-log-type";
const argTopic: &str = "-topic";

#[derive(RustcDecodable, RustcEncodable)]
struct CRequest {
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
    let server = cmdHandler.register(argServer, "127.0.0.1:50005");
    let serverName = cmdHandler.register(argServerName, "tests");
    let serverVersion = cmdHandler.register(argServerVersion, "1.0");
    let serverNo = cmdHandler.register(argServerNo, "1");
    let data = cmdHandler.register(argData, "hello");
    let storageMode = cmdHandler.register(argStorageMode, storageModeFile);
    let logType = cmdHandler.register(argLogType, logTypeMessage);
    let topic = cmdHandler.register(argTopic, "");
    cmdHandler.parse();

    let server = server.borrow();
    let serverName = serverName.borrow();
    let serverVersion = serverVersion.borrow();
    let serverNo = serverNo.borrow();
    let data = data.borrow();
    let storageMode = storageMode.borrow();
    let logType = logType.borrow();
    let topic = topic.borrow();

    let stream = TcpStream::connect(&(*server)).unwrap();
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    {
        let connRequest = CRequest {
            mode: requestModeConnect.to_string(),
            identify: requestIdentifyPublish.to_string(),
            serverName: serverName.to_string(),
            serverVersion: serverVersion.to_string(),
            serverNo: serverNo.to_string(),
            topic: "".to_string(),
            data: "".to_string(),
            storageMode: "".to_string(),
            logType: "".to_string()
        };
        sendRequest(connRequest, stream.try_clone().unwrap());
        /*
        let encoded = json::encode(&connRequest).unwrap();
        let content = vec![encoded, "\n".to_string()].join("");
        writer.write_all(content.as_bytes()).unwrap();
        writer.flush().unwrap();
        */
    }

    loop {
        let pubRequest = CRequest {
            mode: requestModeSending.to_string(),
            identify: requestIdentifyPublish.to_string(),
            serverName: serverName.to_string(),
            serverVersion: serverVersion.to_string(),
            serverNo: serverNo.to_string(),
            topic: topic.to_string(),
            data: data.to_string(),
            storageMode: storageMode.to_string(),
            logType: logType.to_string()
        };
        sendRequest(pubRequest, stream.try_clone().unwrap());
        /*
        let encoded = json::encode(&pubRequest).unwrap();
        let content = vec![encoded, "\n".to_string()].join("");
        writer.write_all(content.as_bytes()).unwrap();
        writer.flush().unwrap();
        */

        thread::sleep_ms(1000);
    }
}
