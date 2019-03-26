pub mod file;

pub trait IStorage {
    fn message(&self, path: &str, content: &str) -> std::io::Result<()> {
        println!("{:?}", content);
        Ok(())
    }
    fn error(&self, path: &str, content: &str) -> std::io::Result<()> {
        println!("{:?}", content);
        Ok(())
    }
}

