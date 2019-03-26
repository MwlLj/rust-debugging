use std::thread;
use std::sync::mpsc;
use std::sync::{
	Arc, Mutex
};
use std::io::prelude::*;

trait IFnBox {
	fn call(self: Box<Self>);
}

impl<F: FnOnce()> IFnBox for F {
	fn call(self: Box<Self>) {
		(*self)();
	}
}

type CJob = Box<IFnBox + Send + 'static>;

struct CWorker {
	handler: thread::JoinHandle<()>,
	threadNo: usize
}

impl CWorker {
	fn new(threadNo: usize, receiver: Arc<Mutex<mpsc::Receiver<CJob>>>) -> CWorker {
		let worker = CWorker {
			handler: thread::spawn(move || {
				loop {
					let f = receiver.lock().unwrap().recv().unwrap();
					// 无法直接调用, 需要间接调用
					// (*f)();
					f.call();
				}
			}),
			threadNo: threadNo
		};
		worker
	}
}

pub struct CThreadPool {
	workers: Vec<CWorker>,
	sender: mpsc::Sender<CJob>
}

impl CThreadPool {
	pub fn new(max: usize) -> CThreadPool {
		assert!(max > 0);
		// create channel
		let (sender, receiver) = mpsc::channel();
		// create receiver Arc
		let receiver = Arc::new(Mutex::new(receiver));
		// create wokers
		let mut workers = Vec::with_capacity(max);
		for i in 0..max {
			let recv = receiver.clone();
			workers.push(CWorker::new(i, recv));
		}
		// new object
		let threadPool = CThreadPool {
			workers: workers,
			sender: sender
		};

		threadPool
	}

	pub fn execute<F>(&self, f: F)
		where F: FnOnce() + Send + 'static {
		self.sender.send(Box::new(f));
	}
}
