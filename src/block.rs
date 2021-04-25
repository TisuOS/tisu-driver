//! # 块设备功能封装
//! 
//! 2021年3月30日 zg

use core::{mem::size_of};
use tisu_memory::{MemoryOp};
use tisu_sync::Bool;
use tisu_sync::SpinMutex;

use crate::{InterruptResult, IoResult, config::{
		InterruptError,
		InterruptOk,
		IoError,
		PAGE_SIZE
	}, pool::Pool, queue::BlockFlag, require::{
		BlockDriver,
		Driver
	}};

use super::{
	header::VirtHeader,
	queue::{
		Header,
		DescFlag,
		VIRTIO_RING_SIZE,
		VirtQueue
	}
};

struct Request {
	pub header: Header,
	pub data:   *mut u8,
	pub status: u8,
	pub lock : SpinMutex,
}

impl Clone for Request{
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            data: self.data,
            status: self.status,
            lock: SpinMutex::new(),
		}
    }
}

impl Copy for Request{}

impl Default for Request {
    fn default() -> Self {
		Self{
		    header: Header::default(),
		    data: 0 as *mut u8,
		    status: 0,
		    lock: SpinMutex::new(),
		}
    }
}

impl Request {
	pub fn new(data:&[u8],offset:usize,write:bool)->Self {
		// let addr = heap.alloc_kernel_memory(size_of::<Request>()).unwrap();
		// let rt = addr as *mut Request;
		// let rq = unsafe {&mut *rt};
		Self {
			header : Header {
				blktype: if write {BlockFlag::Out} else {BlockFlag::In},
				reserved: 0,
				sector: (offset / 512) as u64
			},
			data : data as *const [u8] as *const u8 as *mut u8,
			status : 111,
		    lock: SpinMutex::new(),
		}
		// rq.header.blktype = if write {BlockFlag::Out} else {BlockFlag::In};
		// rq.header.sector = (offset / 512) as u64;
		// rq.header.reserved = 0;
		// rq.data = buffer;
		// rq.status = 111;

		// rt
	}
}

pub struct Block {
	header : &'static mut VirtHeader,
	queue : &'static mut VirtQueue,
	request_pool : Pool<Request>,
	mutex : SpinMutex,
	pub int : Bool,
}

// impl Clone for Block {
//     fn clone(&self) -> Self {
//         Block{
// 			header : unsafe{&mut *(self.header as *const VirtHeader as usize as *mut VirtHeader)},
// 			queue : unsafe{&mut *(self.queue as *const VirtQueue as usize as *mut VirtQueue)},
// 			int : Bool::new(),
//             pin_idx: self.pin_idx,
// 		}
//     }
// }

impl Block {
    pub fn new(header : *mut VirtHeader, memory : &mut impl MemoryOp)->Self {
		let num = (size_of::<VirtQueue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		let queue = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let header = unsafe {&mut *(header)};
		header.set_feature(!(1 << BlockFlag::ReadOnly as u32)).unwrap();
		header.set_ring_size(VIRTIO_RING_SIZE as u32).unwrap();
		header.set_page_size(PAGE_SIZE as u32);
		header.set_pfn(0, (queue as u32) / PAGE_SIZE as u32);
		header.driver_ok();

		let rt = Self {
			header,
			queue : unsafe {&mut *queue},
			request_pool : Pool::default(),
			mutex : SpinMutex::new(),
			int : Bool::new(),
		};
		rt
    }
}

impl Driver for Block {
    fn handler(&mut self)->InterruptResult {
		if !self.int.pop() {return Ok(InterruptOk::Block);}

		while self.queue.is_pending() {
			self.mutex.lock();
			let elem = self.queue.next_elem();
			let rq = self.queue.desc[elem.id as usize].addr as *mut Request;
			if unsafe {(*rq).status != 0} {
				return Err(InterruptError::NoInterrupt);
			}
			unsafe {(*rq).lock.unlock()}
			self.mutex.unlock();
		}
		Ok(InterruptOk::Block)
    }

    fn pending(&mut self)->InterruptResult {
		self.int.set_true();
		Ok(InterruptOk::Block)
    }
}


impl BlockDriver for Block {
	fn sync_write(&mut self, offset : usize, len : usize, data : &[u8])->IoResult {
		self.mutex.lock();
		let idx = self.queue.desc_idx() as usize;
		let v = Request::new(data, offset,true);
		let ori = self.request_pool.get(idx);
		if ori.status != 0 {
			return Err(IoError::RequestError);
		}
		let rq = self.request_pool.replace_ref(idx, v);
		let header = &rq.header as *const Header;
		let status = &rq.status as *const u8;
		let mut flag = DescFlag::Next as u16;
		self.queue.add_avail();
		self.queue.add_desc(header as u64,size_of::<Header>() as u32,flag);
		self.queue.add_desc(data as *const [u8] as *const u8 as u64, len as u32, flag);
		flag = DescFlag::Write as u16;
		self.queue.add_desc(status as u64, 1, flag);
		self.mutex.unlock();
		self.header.notify();
		rq.lock.lock();
		self.header.notify();
		rq.lock.lock();
		rq.lock.unlock();
		Ok(())
		// free(rq as *mut u8);
	}

	fn sync_read(&mut self, offset : usize, len : usize, data : &mut [u8])->IoResult {
		self.mutex.lock();
		let idx = self.queue.desc_idx() as usize;
		let v = Request::new(data,offset,false);
		let ori = self.request_pool.get(idx);
		if ori.status != 0 {
			return Err(IoError::Info(&"Request pool is full"));
		}
		let rq = self.request_pool.replace_ref(idx, v);
		let header = &rq.header as *const Header;
		let status = &rq.status as *const u8;
		let mut flag = DescFlag::Next as u16;
		self.queue.add_avail();
		self.queue.add_desc(header as u64,size_of::<Header>() as u32,flag);
		flag |= DescFlag::Write as u16;
		self.queue.add_desc(data as *const [u8] as *const u8 as u64, len as u32, flag);
		flag = DescFlag::Write as u16;
		self.queue.add_desc(status as u64, 1, flag);
		self.mutex.unlock();
		rq.lock.lock();
		self.header.notify();
		rq.lock.lock();
		rq.lock.unlock();
		Ok(())
		// free(rq as *mut u8);
	}
}
