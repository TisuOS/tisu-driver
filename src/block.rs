//! # 块设备功能封装
//! 
//! 2021年3月30日 zg

use core::{mem::size_of, ptr::slice_from_raw_parts};
use tisu_memory::{MemoryOp};
use tisu_sync::Bool;
use tisu_sync::Mutex;

use crate::{config::PAGE_SIZE, queue::BlockFlag, require::{BlockDriver, Driver}};

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
	pub lock : Mutex,
}

impl Clone for Request{
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            data: self.data,
            status: self.status,
            lock: Mutex::new(),
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
		    lock: Mutex::new(),
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
	request_pool : &'static mut [Request],
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
    pub fn new(virtio_addr : usize, memory : &mut impl MemoryOp)->Self {
		let num = (size_of::<VirtQueue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		let queue = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let header = unsafe {&mut *(virtio_addr as *mut VirtHeader)};
		header.set_feature(!(1 << BlockFlag::ReadOnly as u32)).unwrap();
		header.set_ring_size(VIRTIO_RING_SIZE as u32).unwrap();
		header.set_page_size(PAGE_SIZE as u32);
		header.set_pfn(0, (queue as u32) / PAGE_SIZE as u32);
		header.driver_ok();
		let addr = memory.kernel_page(
			VIRTIO_RING_SIZE * size_of::<Request>() / PAGE_SIZE + 1
		).unwrap() as *mut Request;
		let addr = slice_from_raw_parts(addr, VIRTIO_RING_SIZE);

		let rt = Self {
			header,
			queue : unsafe {&mut *queue},
			request_pool : unsafe {&mut *(addr as *mut [Request])},
			int : Bool::new(),
		};
		rt
    }
}

impl Driver for Block {
    fn handler(&mut self) {
		if !self.int.pop() {return;}

		while self.queue.is_pending() {
			let elem = self.queue.next_elem();
			let rq = self.queue.desc[elem.id as usize].addr as *mut Request;
			unsafe {(*rq).lock.unlock()}
		}
    }

    fn pending(&mut self) {
		self.int.set_true();
    }
}


impl BlockDriver for Block {
	fn sync_write(&mut self, offset : usize, len : usize, data : &[u8]) {
		let idx = self.queue.desc_idx() as usize;
		self.request_pool[idx] = Request::new(data, offset,true);
		let rq = &mut self.request_pool[idx];
		let header = &rq.header as *const Header;
		let status = &rq.status as *const u8;
		let mut flag = DescFlag::Next as u16;
		self.queue.add_avail();
		self.queue.add_desc(header as u64,size_of::<Header>() as u32,flag);
		self.queue.add_desc(data as *const [u8] as *const u8 as u64, len as u32, flag);
		flag = DescFlag::Write as u16;
		self.queue.add_desc(status as u64, 1, flag);
		self.header.notify();
		rq.lock.lock();
		self.header.notify();
		rq.lock.lock();
		rq.lock.unlock();
		// free(rq as *mut u8);
	}

	fn sync_read(&mut self, offset : usize, len : usize, data : &mut [u8]) {
		let idx = self.queue.desc_idx() as usize;
		self.request_pool[idx] = Request::new(data,offset,false);
		let rq = &mut self.request_pool[idx];
		let header = &rq.header as *const Header;
		let status = &rq.status as *const u8;
		let mut flag = DescFlag::Next as u16;
		self.queue.add_avail();
		self.queue.add_desc(header as u64,size_of::<Header>() as u32,flag);
		flag |= DescFlag::Write as u16;
		self.queue.add_desc(data as *const [u8] as *const u8 as u64, len as u32, flag);
		flag = DescFlag::Write as u16;
		self.queue.add_desc(status as u64, 1, flag);
		rq.lock.lock();
		self.header.notify();
		rq.lock.lock();
		rq.lock.unlock();
		// free(rq as *mut u8);
	}
}
