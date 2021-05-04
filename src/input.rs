//! # 输入设备
//! 
//! 2021年3月30日

#![allow(dead_code)]
use core::mem::size_of;

use tisu_memory::MemoryOp;

use crate::{
    Driver,
    InterruptError,
    InterruptOk,
    InterruptResult,
    VirtHeader,
    VirtQueue,
    config::{PAGE_SIZE},
    queue::{DescFlag, VIRTIO_F_RING_EVENT_IDX, VIRTIO_RING_SIZE}
};

#[repr(C)]
pub struct InputABSInfo{
    min : u32,
    max : u32,
    fuzz : u32,
    flat : u32,
    res : u32,
}
#[repr(C)]
struct InputDevids{
    bustype : u16,
    vendor : u16,
    product : u16,
    version : u16,
}
#[repr(C)]
struct InputConfig{
    select : u8,
    subsel : u8,
    size : u8,
    reserved : [u8;5],
    ctype : ConfigType,
}
#[repr(C)]
enum ConfigType{
    S([u8;128]),
    Bitmap([u8;128]),
    Info(InputABSInfo),
    Ids(InputDevids),
}
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct InputEvent{
    pub etype : EventType,
    pub code : u16,
    pub value : u32,
}

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum EventType {
    Syn = 0x00,
    Key = 0x01,
    Rel = 0x02,
    Abs = 0x03,
    Msc = 0x04,
    Sw = 0x05,
    Led = 0x11,
    Snd = 0x12,
    Rep = 0x14,
    Ff = 0x15,
    Pwr = 0x16,
    FfStatus = 0x17,
    Max = 0x1f,
}
pub struct InputDevice{
    buffer : *mut InputEvent,
	event_queue : &'static mut VirtQueue, // 0
	status_queue : &'static mut VirtQueue, // 1
    header : &'static mut VirtHeader,
    // abs_event : VecDeque<InputEvent>,
    // key_event : VecDeque<InputEvent>,
}

const EVENT_BUFFER_SIZE : usize = 128;

impl InputDevice {
    /// ## 新建输入设备管理
    /// 负责初始化状态队列、事件队列
    pub fn new(header : *mut VirtHeader, memory : &mut impl MemoryOp) ->Self {
		let num = (size_of::<VirtQueue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		let eq = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let sq = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let header = unsafe {&mut *header};
		header.set_feature(!(1 << VIRTIO_F_RING_EVENT_IDX)).unwrap();
		header.set_ring_size(VIRTIO_RING_SIZE as u32).unwrap();
		header.set_page_size(PAGE_SIZE as u32);
		header.set_pfn(0, (eq as u32) / PAGE_SIZE as u32);
		header.set_pfn(1, (sq as u32) / PAGE_SIZE as u32);
		header.driver_ok();
        let buffer = memory.alloc_memory(
            size_of::<InputEvent>()*EVENT_BUFFER_SIZE,true).unwrap();
        let mut rt = Self{
            buffer: buffer as *mut InputEvent,
            event_queue: unsafe{&mut *eq},
            status_queue: unsafe{&mut *sq},
            header: header,
            // abs_event: VecDeque::with_capacity(10),
            // key_event: VecDeque::with_capacity(10),
        };
        for i in 0..EVENT_BUFFER_SIZE {
            rt.fill_event(i);
        }
        rt
    }

    fn add_abs(&mut self, _event : &InputEvent) {
        // self.abs_event.push_back(*event);
        // if self.abs_event.len() >= 2{
        //     let e = self.abs_event.pop_front().unwrap();
        //     let x = e.value as f32 / 32767.0;
        //     let e = self.abs_event.pop_front().unwrap();
        //     let y = e.value as f32 / 32767.0;
        //     self.abs_event.pop_front();
        //     // add_mouse_position(ScalePoint{x:x,y:y});
        // }
    }

    fn add_key(&mut self, _event : &InputEvent) {
        // self.key_event.push_back(*event);
        // if self.key_event.len() >= 1{
        //     let e = self.key_event.pop_front().unwrap();
        //     if e.value == 1{
        //         // add_key_press(e.code);
        //     }
        //     else if e.value == 0{
        //         // add_key_release(e.code);
        //     }
        // }
    }

    fn fill_event(&mut self, buffer_idx : usize) {
        let addr = unsafe {self.buffer.add(buffer_idx % EVENT_BUFFER_SIZE) as u64};
        let size = size_of::<InputEvent>() as u32;
        self.event_queue.add_avail();
        self.event_queue.add_desc(addr, size, DescFlag::Write as u16);
    }
}


impl Driver for InputDevice {
    /// 通过循环调用此函数获取输入值
    fn handler(&mut self)->InterruptResult {
        let mut rt = Ok(InterruptOk::Null);
        while self.event_queue.is_pending() {
            let ref elem = self.event_queue.next_elem();
            let ref desc = self.event_queue.desc[elem.id as usize];
            let event = unsafe {(desc.addr as *const InputEvent).as_ref().unwrap()};
            self.fill_event(elem.id as usize);
            if event.code == 0 && event.value == 0 {
                continue;
            }
            rt = Ok(InterruptOk::Input(*event));
            break;
        }
        rt
    }

    /// 此函数仅处理 status 事件
    fn pending(&mut self)->InterruptResult {
        let mut rt = Err(InterruptError::NoInterrupt);
        if self.event_queue.is_pending() {
            rt = Ok(InterruptOk::Null);
        }
        else if self.status_queue.is_pending() {
            let ref elem = self.status_queue.next_elem();
            let ref desc = self.status_queue.desc[elem.id as usize];
            let event = unsafe {(desc.addr as *const InputEvent).as_ref().unwrap()};
            rt = Ok(InterruptOk::Input(*event));
        }
        rt
    }
}

