extern crate rust_parse;
extern crate log_server;

use std::rc::Rc;

use log_server::dispatch::connect;
use rust_parse::cmd::CCmd;

const argServerHost: &str = "-host";
const argServerPort: &str = "-port";
const argThreadMax: &str = "-thread-max";

fn printHelp() {
    let mut message = String::new();
    message.push_str("options: \n");
    message.push_str("\t-host: server listen ip, exp: 0.0.0.0\n");
    message.push_str("\t-port: server listen port, exp: 50005\n");
    message.push_str("\t-thread-max: thread max number, exp: 10\n");
    print!("{}", message);
}

fn main() {
    printHelp();

    let mut cmdHandler = CCmd::new();
    let host = cmdHandler.register(argServerHost, "0.0.0.0");
    let port = cmdHandler.register(argServerPort, "50005");
    let threadMax = cmdHandler.register(argThreadMax, "10");
    cmdHandler.parse();

    let host = host.borrow();
    let port = port.borrow();
    let threadMax = threadMax.borrow();

    let mut server = String::new();
    server.push_str(&(*host));
    server.push_str(":");
    server.push_str(&(*port));

    if let Ok(threadMax) = threadMax.trim().parse() {
        let connect = connect::CConnect::new(threadMax);
        connect.start(&server);
    } else {
        println!("threadMax must be number!!!");
    }
}
