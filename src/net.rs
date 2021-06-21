use core::mem::size_of;

use tisu_memory::MemoryOp;

use crate::{Driver, InterruptOk, InterruptResult, VirtHeader, VirtQueue, config::PAGE_SIZE, pool::Pool, queue::{DescFlag, VIRTIO_RING_SIZE}, require::NetDriver};

#[allow(dead_code)]
pub struct Net {
    receive : &'static mut VirtQueue,
    send : &'static mut VirtQueue,
    header : &'static mut VirtHeader,
    send_header : Pool<NetHeader>,
    receive_header : Pool<NetHeader>,
}

impl Net {
    pub fn new(header : *mut VirtHeader, memory : &mut impl MemoryOp)->Self {
		let num = (size_of::<VirtQueue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		let receive = memory.kernel_page(num).unwrap() as *mut VirtQueue;
        let send = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let header = unsafe {&mut *(header)};
		header.set_feature(!(Feature::MQ.v())).unwrap();
		header.set_ring_size(VIRTIO_RING_SIZE as u32).unwrap();
		header.set_page_size(PAGE_SIZE as u32);
		header.set_pfn(0, (receive as u32) / PAGE_SIZE as u32);
        header.set_pfn(1, (send as u32) / PAGE_SIZE as u32);
		header.driver_ok();
        Self {
            receive : unsafe {&mut *receive},
            send : unsafe {&mut *send},
            header,
            send_header : Pool::default(),
            receive_header : Pool::default(),
        }
    }
}

impl NetDriver for Net {
    fn send(&mut self, data : &[u8]) {
        let mut flag = DescFlag::Next as u16;
        let header = self.send_header.get(self.send.desc_idx() as usize);
        self.send.add_avail();
        let len = size_of::<NetHeader>() as u32;
        self.send.add_desc(header as *mut NetHeader as *mut u8 as u64, len, flag);
        flag = DescFlag::Write as u16;
        self.send.add_desc(data as *const [u8] as *const u8 as u64, data.len() as u32, flag);
        self.header.notify(1);
    }

    fn mac(&self)->usize {
        let config = self.header.config_address() as *const Config;
        unsafe {
            let mut mac = 0;
            for i in 0..6 {
                mac = (mac << 8) | (*config).mac[i] as usize;
            }
            mac
        }
    }
}

impl Driver for Net {
    fn handler(&mut self)->crate::InterruptResult {
        InterruptResult::Ok(InterruptOk::Net)
    }

    fn pending(&mut self)->crate::InterruptResult {
        InterruptResult::Ok(InterruptOk::Net)
    }
}

#[repr(u32)]
pub enum Feature {
    MQ = 22,
}

impl Feature {
    pub fn v(self)->u32 {
        self as u32
    }
}

#[derive(Clone, Copy, Default)]
#[allow(dead_code)]
pub struct NetHeader {
    ntype : u8,
    len : u16,
    size : u16,
    csum_start : u16,
    csum_offset : u16,
    num_buffer : u16,
}

#[allow(dead_code)]
pub struct Config {
    mac : [u8;6],
    status : u16,
    max_queue_pairs : u16,
    mtu : u16,
}

