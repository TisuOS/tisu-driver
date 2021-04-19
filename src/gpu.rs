//! # 显示驱动
//! 
//! 2021年3月30日

#![allow(dead_code)]
use core::{cmp::min, mem::size_of};
use tisu_memory::MemoryOp;
use tisu_sync::{Bool, SpinMutex};
use super::{header::VirtHeader, queue::{DescFlag, VIRTIO_RING_SIZE, VirtQueue}};
use crate::{GraphicResult, InterruptResult, config::{GraphicError, InterruptOk, PAGE_SIZE, Pixel, Rect}, pool::Pool};
use crate::require::GraphicDriver;
use crate::Driver;

pub struct GPU{
	header : &'static mut VirtHeader,
	queue : &'static mut VirtQueue,
    frame_buffer : *mut Pixel,
    res_pool : Pool<ResourceFlush>,
    trans_pool : Pool<TransferToHost2d>,
    scan_pool : Pool<Scanout>,
    create_pool : Pool<Create2D>,
    attach_pool : Pool<AttachBacking>,
    entry_pool : Pool<MemEntry>,
    header_pool : Pool<ControllHeader>,
    width : usize,
    height : usize,
    mutex : SpinMutex,
    int : Bool,
}

impl GPU {
    pub fn new(
            header : *mut VirtHeader,
            width : usize,
            height : usize,
            memory : &'static mut dyn MemoryOp,
        )->Self{
		let num = (size_of::<VirtQueue>() + PAGE_SIZE - 1) / PAGE_SIZE;
		let queue = memory.kernel_page(num).unwrap() as *mut VirtQueue;
		let header = unsafe {&mut *header};
        let num = (width * height * size_of::<Pixel>() + PAGE_SIZE - 1) / PAGE_SIZE;
		header.set_feature(!0).unwrap();
		header.set_ring_size(VIRTIO_RING_SIZE as u32).unwrap();
		header.set_page_size(PAGE_SIZE as u32);
		header.set_pfn(0, (queue as u32) / PAGE_SIZE as u32);
		header.driver_ok();

        let mut rt = Self{
			header,
			queue : unsafe {&mut *queue},
            frame_buffer: memory.kernel_page(num).unwrap() as *mut Pixel,
            res_pool : Pool::default(),
            trans_pool : Pool::default(),
            scan_pool : Pool::default(),
            create_pool : Pool::default(),
            attach_pool : Pool::default(),
            entry_pool : Pool::default(),
            header_pool : Pool::default(),
            width,
            height,
            mutex: SpinMutex::new(),
            int: Bool::new(),
        };
        rt.reset();
        rt
    }
    /// 清空屏幕 rgba（10，10，10，255）
    fn reset(&mut self){
        let rect = Rect{x1:0,y1:0,x2:self.width as u32,y2:self.height as u32};
        self.fill_rect(0, 0, self.width, self.height, Pixel{r:10,g:10,b:10,a:255});
        self.create_resouce_id(self.width, self.height, 1);
        self.attach(1);
        self.set_scanout(rect.clone(), 1, 0);
        self.transfer(rect.clone(), 1);
        self.flush(rect.clone(), 1);
    }

    /// 刷新 resouce 里的区域
    pub fn flush(&mut self, rect : Rect, resource_idx : usize){
        let flush = ResourceFlush::new(rect, resource_idx);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.res_pool.replace_u64(idx, flush);
        self.add_desc::<ResourceFlush>(addr,ControllType::ResourceFlush);
    }

    /// 将 source 对应区域传输给 GPU
    pub fn transfer(&mut self, rect : Rect, resource_idx : usize){
        let trans = TransferToHost2d::new(rect, resource_idx);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.trans_pool.replace_u64(idx, trans);
        self.add_desc::<TransferToHost2d>(addr,ControllType::TransferToHost2d);
    }
    /// 将 source 和 scanout 中的某个区域绑定
    pub fn set_scanout(&mut self, rect : Rect, resource_idx : usize, scanout_idx : usize){
        let scan = Scanout::new(rect, resource_idx, scanout_idx);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.scan_pool.replace_u64(idx, scan);
        self.add_desc::<Scanout>(addr,ControllType::SetScanout);
    }
    /// 创建一个 source，设定好宽、高
    pub fn create_resouce_id(&mut self, width : usize, height : usize, resource_idx : usize){
        let create = Create2D::new(width, height, resource_idx);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.create_pool.replace_u64(idx, create);
        self.add_desc::<Create2D>(addr,ControllType::ResourceCreate2d);
    }

    fn add_desc<T>(&mut self, addr1 : u64, ctype : ControllType) {
        let header = ControllHeader::default_val(ctype);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.header_pool.replace_u64(idx, header);
        let ref mut q = self.queue;
        q.add_avail();
        q.add_desc(addr1, size_of::<T>() as u32, DescFlag::Next as u16);
        q.add_desc(addr, size_of::<ControllHeader>() as u32,
        DescFlag::Write as u16);
    }

    /// 将 source 与某块内存绑定
    pub fn attach(&mut self, resource_idx : u32){
        let at = AttachBacking::new(resource_idx, resource_idx);
        let entry = MemEntry::new(self.frame_buffer as u64,
            (self.width * self.height * size_of::<Pixel>()) as u32);
        let ctype = ControllType::ResourceAttachBacking;
        let header = ControllHeader::default_val(ctype);
        let idx = self.queue.desc_idx() as usize;
        let addr1 = self.attach_pool.replace_u64(idx, at);
        self.queue.add_avail();
        self.queue.add_desc(addr1, size_of::<AttachBacking>() as u32,
        DescFlag::Next as u16);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.entry_pool.replace_u64(idx, entry);
        self.queue.add_desc(addr, size_of::<MemEntry>() as u32,
        DescFlag::Next as u16);
        let idx = self.queue.desc_idx() as usize;
        let addr = self.header_pool.replace_u64(idx, header);
        self.queue.add_desc(addr, size_of::<ControllHeader>() as u32,
        DescFlag::Write as u16);
    }

    /// 发送 QueueNotify
    fn run(&mut self){
        self.header.notify();
    }

    fn fill_rect(&mut self, x : usize, y : usize, width : usize, height : usize, color : Pixel){
        for i in x..(x + width){
            for j in y..(y + height){
                unsafe {
                    let idx = j * self.width + i;
                    self.frame_buffer.add(idx).write_volatile(color);
                }
            }
        }
    }
}

impl Driver for GPU {
    fn handler(&mut self)->InterruptResult {
        if !self.int.pop() {
            return Ok(InterruptOk::Graphic);
        }
        unsafe {
            while self.queue.is_pending() {
                let elem = self.queue.next_elem();
                let idx = elem.id as usize;
                if (*(self.queue.desc[idx].addr as *const ControllHeader)).ctype != ControllType::RespOkNoData{
                    panic!("GPU Err {:?}", (*(self.queue.desc[idx].addr as *const ControllHeader)).ctype);
                }
            }
            self.mutex.unlock();
        }
        Ok(InterruptOk::Graphic)
    }

    fn pending(&mut self)->InterruptResult {
        self.int.set_true();
        Ok(InterruptOk::Graphic)
    }
}

impl GraphicDriver for GPU {
    fn draw_blend(&mut self, rect : Rect, buffer: &[Pixel])->GraphicResult {
        let x1 = rect.x1 as usize;
        let y1 = rect.y1 as usize;
        let x2 = rect.x2 as usize;
        let y2 = rect.y2 as usize;
        if x1 >= self.width || y1 > self.height{
            return Err(GraphicError::InvalidRect(rect));
        }
        let st = y1 * self.width;
        let ed = min(y2, self.height) * self.width;
        let ptr = &buffer[0] as *const Pixel;
        let mut idx = 0;
        self.mutex.lock();
        let t = 1.0 / 255.0;
        for y in (st..ed).step_by(self.width){
            for x in x1..min(x2, self.width){
                unsafe {
                    let id = x + y;
                    let color1 = *ptr.add(idx);
                    let color2 = *self.frame_buffer.add(id);
                    let rate =  color1.a as f32 * t;
                    let rate2 = 1.0 - rate;
                    let color = Pixel{
                        r : (color1.r as f32 * rate) as u8 + (color2.r as f32 * rate2) as u8,
                        g : (color1.g as f32 * rate) as u8 + (color2.g as f32 * rate2) as u8,
                        b : (color1.b as f32 * rate) as u8 + (color2.b as f32 * rate2) as u8,
                        a : (color1.a as f32 * rate) as u8 + (color2.a as f32 * rate2) as u8,
                    };
                    self.frame_buffer.add(id).write_volatile(color);
                    idx += 1;
                }
            }
        }
        self.mutex.unlock();
        Ok(())
    }

    fn draw_override(&mut self, rect : Rect, buffer: &[Pixel])->GraphicResult {
        let x1 = rect.x1 as usize;
        let y1 = rect.y1 as usize;
        let x2 = rect.x2 as usize;
        let y2 = rect.y2 as usize;
        if x1 >= self.width || y1 > self.height{
            return Err(GraphicError::InvalidRect(rect));
        }
        let st = y1 * self.width;
        let ed = min(y2, self.height) * self.width;
        let ptr = &buffer[0] as *const Pixel;
        let mut idx;
        let mut row = 0;
        let width = x2 - x1;
        let line = min(x2, self.width) - x1;
        self.mutex.lock();
        for y in (st..ed).step_by(self.width){
            idx = row * width;
            unsafe {
                self.frame_buffer.add(y + x1).copy_from(ptr.add(idx), line);
            }
            row += 1;
        }
        self.mutex.unlock();
        Ok(())
    }

    fn refresh(&mut self) {
        let rect = Rect{x1:0, y1:0, x2:self.width as u32, y2:self.height as u32};
        self.transfer(rect.clone(), 1);
        self.flush(rect, 1);
        self.run();
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ResourceFlush {
	header : ControllHeader,
	rect : Rect,
	resource_id : u32,
	padding : u32,
}
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ControllHeader{
    ctype : ControllType,
    flag : u32,
    fence_idx : u64,
    ctx_id : u32,
    padding : u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct Scanout {
	header: ControllHeader,
	rect: Rect,
	scanout_id: u32,
	resource_id: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Create2D{
    header : ControllHeader,
    resource_idx : u32,
    format : PixelFormat,
    width : u32,
    height : u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct TransferToHost2d {
	header: ControllHeader,
	rect: Rect,
	offset: u64,
	resource_id: u32,
	padding: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct AttachBacking {
	header: ControllHeader,
	resource_id: u32,
	entries: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MemEntry {
	addr: u64,
	length: u32,
	padding: u32,
}

#[repr(u32)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ControllType {
    GetDisplayInfo = 0x0100,
	ResourceCreate2d,
	ResourceUref,
	SetScanout,
	ResourceFlush,
	TransferToHost2d,
	ResourceAttachBacking,
	ResourceDetachBacking,
	GetCapsetInfo,
	GetCapset,
	GetEdid,
	// cursor commands
	UpdateCursor = 0x0300,
	MoveCursor,
	// success responses
	RespOkNoData = 0x1100,
	RespOkDisplayInfo,
	RespOkCapsetInfo,
	RespOkCapset,
	RespOkEdid,
	// error responses
	RespErrUnspec = 0x1200,
	RespErrOutOfMemory,
	RespErrInvalidScanoutId,
	RespErrInvalidResourceId,
	RespErrInvalidContextId,
	RespErrInvalidParameter,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy)]
enum PixelFormat {
    B8G8R8A8 = 1,
    B8G8R8X8 = 2,
    A8R8G8B8 = 3,
    X8R8G8B8 = 4,
    R8G8B8A8 = 67,
    X8B8G8R8 = 68,
    A8B8G8R8 = 121,
    R8G8B8X8 = 134,
}

impl Create2D {
    pub fn new(width : usize, height : usize, resouce_idx : usize)->Self {
        Self {
            header: ControllHeader::default_val(ControllType::ResourceCreate2d),
            resource_idx: resouce_idx as u32,
            format: PixelFormat::R8G8B8A8,
            width: width as u32,
            height: height as u32,
        }
    }
}

impl Default for Create2D {
    fn default() -> Self {
        Self {
            header: ControllHeader::default(),
            resource_idx: 0,
            format: PixelFormat::A8B8G8R8,
            width: 0,
            height: 0,
        }
    }
}

impl ControllHeader {
    pub fn default_val(ctype : ControllType)->Self {
        Self {
            ctype,
            flag: 0,
            fence_idx: 0,
            ctx_id: 0,
            padding: 0,
        }
    }
}

impl Default for ControllHeader {
    fn default()->Self {
        Self {
            ctype : ControllType::ResourceFlush,
            flag: 0,
            fence_idx: 0,
            ctx_id: 0,
            padding: 0,
        }
    }
}

impl TransferToHost2d {
    pub fn new(rect : Rect, resouce_idx : usize)->Self {
        Self {
            header: ControllHeader::default_val(ControllType::TransferToHost2d),
            rect,
            offset: 0,
            resource_id: resouce_idx as u32,
            padding: 0,
        }
    }
}

impl AttachBacking {
    pub fn new(resource_idx : u32, entries : u32) ->Self {
        Self {
            header: ControllHeader::default_val(ControllType::ResourceAttachBacking),
            resource_id: resource_idx as u32,
            entries,
        }
    }
}

impl Scanout {
    pub fn new(rect : Rect, resource_idx : usize, scanout_idx : usize)->Self {
        Self {
            header: ControllHeader::default_val(ControllType::SetScanout),
            rect,
            scanout_id: scanout_idx as u32,
            resource_id: resource_idx as u32,
        }
    }
}

impl ResourceFlush {
    pub fn new(rect : Rect, resouce_idx : usize)->Self {
        Self {
            header: ControllHeader::default_val(ControllType::ResourceFlush),
            rect,
            resource_id : resouce_idx as u32,
            padding: 0,
        }
    }
}

impl MemEntry {
    pub fn new(addr : u64, length : u32)->Self {
        Self{
            addr,
            length,
            padding: 0,
        }
    }
}