//! # 虚拟队列
//! 
//! 2021年3月29日 zg

#![allow(dead_code)]
use core::mem::size_of;

use crate::config::PAGE_SIZE;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum BlockFlag {
	In = 0,
	Out = 1,
	Flush = 4,
	Discard = 11,
	WriteZeros = 13,
	ReadOnly = 5,
}

pub const VIRTIO_RING_SIZE : usize = 1 << 7;
pub const VIRTIO_F_RING_EVENT_IDX : u32 = 29;
const VIRTIO_AVAIL_F_NO_INTERRUPT: u16 = 1;
const VIRTIO_USED_F_NO_NOTIFY: u16 = 1;

#[repr(C)]
pub struct VirtQueue {
	pub desc:  [Descriptor; VIRTIO_RING_SIZE],
	pub avail: Available,
	pub padding0: [u8; PAGE_SIZE - size_of::<Descriptor>() * VIRTIO_RING_SIZE - size_of::<Available>()],
	pub used:     Used,
    desc_idx : u16,
	used_idx : u16,
}

#[repr(C)]
#[derive(Debug)]
pub struct Descriptor {
	pub addr:  u64,
	pub len:   u32,
	pub flags: u16,
	pub next:  u16,
}

#[repr(C)]
pub struct Available {
	pub flags: u16,
	pub idx:   u16,
	pub ring:  [u16; VIRTIO_RING_SIZE],
	pub event: u16,
}

#[repr(C)]
pub struct Used {
	pub flags: u16,
	pub idx:   u16,
	pub ring:  [UsedElem; VIRTIO_RING_SIZE],
	pub event: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct UsedElem {
	pub id:  u32,
	pub len: u32,
}

pub enum DescFlag {
	Next = 1,
	Write = 2,
	Indirect = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Header {
	pub blktype:  BlockFlag,
	pub reserved: u32,
	pub sector:   u64,
}

impl Header {
	pub fn default()->Self {
		Self{
			blktype : BlockFlag::In,
			reserved : 0,
			sector : 0,
		}
	}
}

impl VirtQueue {
    pub fn add_desc(&mut self, addr : u64, len : u32, flag : u16) {
        let next;
        if flag & DescFlag::Next as u16 == 0 {next = 0}
        else {next = (self.desc_idx + 1) % VIRTIO_RING_SIZE as u16}

        let desc = Descriptor {
            addr,
            len,
            flags: flag,
            next,
        };
        self.desc[self.desc_idx as usize] = desc;
        self.desc_idx = (self.desc_idx + 1) % VIRTIO_RING_SIZE as u16;
    }

	pub fn add_avail(&mut self) {
		self.avail.ring[self.avail.idx as usize % VIRTIO_RING_SIZE] = self.desc_idx;
		self.avail.idx = self.avail.idx.wrapping_add(1);
	}

	pub fn is_pending(&self)->bool {
		self.used_idx != self.used.idx
	}

	pub fn next_elem(&mut self)->UsedElem {
		let elem = self.used.ring[self.used_idx as usize % VIRTIO_RING_SIZE];
		self.used_idx = self.used_idx.wrapping_add(1);
		elem
	}

	pub fn desc_idx(&self)->u16 {
		self.desc_idx
	}
}


