extern crate rust_parse;
extern crate log_server;

use std::rc::Rc;

use log_server::dispatch::connect;
use rust_parse::cmd::CCmd;

const argServer: &str = "-server";
const argThreadMax: &str = "-thread-max";

fn main() {
    let mut cmdHandler = CCmd::new();
    let server = cmdHandler.register(argServer, "0.0.0.0:50005");
    let threadMax = cmdHandler.register(argThreadMax, "10");
    cmdHandler.parse();

    let server = server.borrow();
    let threadMax = threadMax.borrow();

    let connect = connect::CConnect::new(threadMax.trim().parse().unwrap());
    connect.start(&server);
}
