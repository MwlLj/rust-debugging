extern crate rust_parse;
extern crate log_sub;
extern crate rustc_serialize;

use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::net::TcpStream;
use std::{thread, time};

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

const argHelp: &str = "-help";
const argServer: &str = "-server";
const argServerName: &str = "-server-name";
const argServerVersion: &str = "-server-version";
const argServerNo: &str = "-server-no";
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

fn main() {
    let mut cmdHandler = CCmd::new();
    let help = cmdHandler.register(argHelp, "");
    let server = cmdHandler.register(argServer, "127.0.0.1:50005");
    let serverName = cmdHandler.register(argServerName, "tests");
    let serverVersion = cmdHandler.register(argServerVersion, "1.0");
    let serverNo = cmdHandler.register(argServerNo, "1");
    let topic = cmdHandler.register(argTopic, "");
    let logType = cmdHandler.register(argLogType, "");
    cmdHandler.parse();

    let help = help.borrow();
    let server = server.borrow();
    let serverName = serverName.borrow();
    let serverVersion = serverVersion.borrow();
    let serverNo = serverNo.borrow();
    let topic = topic.borrow();
    let logType = logType.borrow();

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
                    identify: requestIdentifySubscribe.to_string(),
                    serverName: serverName.to_string(),
                    serverVersion: serverVersion.to_string(),
                    serverNo: serverNo.to_string(),
                    topic: topic.to_string(),
                    data: "".to_string(),
                    storageMode: "".to_string(),
                    logType: logType.to_string()
                };
                let encoded = json::encode(&connRequest).unwrap();
                let content = vec![encoded, "\n".to_string()].join("");
                writer.write_all(content.as_bytes()).unwrap();
                writer.flush().unwrap();

                let mut buffer = String::new();
                while let Ok(size) = reader.read_line(&mut buffer) {
                    if size > 0 {
                        if buffer != "\n".to_string() {
                            print!("{}", buffer);
                        }
                        buffer.clear();
                    }
                }
                println!("closed");
            } else {
                thread::sleep(time::Duration::from_millis(3000));
            }
        }
    }
}
