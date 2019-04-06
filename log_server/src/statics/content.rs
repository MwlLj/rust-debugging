extern crate chrono;

use chrono::prelude::*;

pub struct CContent {
}

impl CContent {
    fn now(&self) -> String {
        let dt = Local::now();
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn full(&self, root: &str, contentType: &str, topic: &str, content: &str) -> String {
        let mut result = String::new();
        result.push_str("[");
        result.push_str(contentType);
        result.push_str("] [");
        result.push_str(&self.now());
        result.push_str("] [");
        result.push_str(topic);
        result.push_str("] ");
        result.push_str(content);
        result
    }
}

impl CContent {
    pub fn new() -> CContent {
        CContent{}
    }
}

