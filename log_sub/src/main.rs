extern crate rust_parse;
extern crate log_sub;
extern crate rustc_serialize;

use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::net::TcpStream;

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
const argStorageMode: &str = "-storage-mode";
const argLogType: &str = "-log-type";

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

fn main() {
    let mut cmdHandler = CCmd::new();
    let server = cmdHandler.register(argServer, "127.0.0.1:50005");
    let serverName = cmdHandler.register(argServerName, "tests");
    let serverVersion = cmdHandler.register(argServerVersion, "1.0");
    let serverNo = cmdHandler.register(argServerNo, "1");
    cmdHandler.parse();

    let server = server.borrow();
    let serverName = serverName.borrow();
    let serverVersion = serverVersion.borrow();
    let serverNo = serverNo.borrow();

    let stream = TcpStream::connect(&(*server)).unwrap();
    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    let connRequest = CRequest {
        mode: requestModeConnect.to_string(),
        identify: requestIdentifySubscribe.to_string(),
        serverName: serverName.to_string(),
        serverVersion: serverVersion.to_string(),
        serverNo: serverNo.to_string(),
        topic: "".to_string(),
        data: "".to_string(),
        storageMode: "".to_string(),
        logType: "".to_string()
    };
    let encoded = json::encode(&connRequest).unwrap();
    let content = vec![encoded, "\n".to_string()].join("");
    writer.write_all(content.as_bytes()).unwrap();
    writer.flush().unwrap();

    loop {
        // let mut line = String::new();
        // reader.read_line(&mut line).unwrap();
        // println!("{}", line);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            println!("{:?}", buffer);
            buffer.clear();
        }
    }
}
