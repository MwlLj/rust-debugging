pub mod file;

pub trait IStorage {
    fn write(&mut self, path: &str, logType: &str, content: &str) -> std::io::Result<()> {
        println!("{:?}", content);
        Ok(())
    }
}

