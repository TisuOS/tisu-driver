//! # 驱动对外接口要求
//! 
//! 2021年4月14日 zg

use tisu_memory::{AutoMemory, PageOp};

pub trait DriverInterrupt {
    fn new(virtio_addr : usize, page : &impl PageOp, page_size : usize)->Self;
    /// 处理中断
    fn handler(&mut self);
    /// 通知中断
    fn pending(&mut self);
}

pub trait BlockDriver : DriverInterrupt {
    fn write(&mut self, blk_idx : usize, idx : usize, offset : usize, len : usize, data : &impl AutoMemory<u8>);
    fn read(&mut self, blk_idx : usize, idx : usize, offset : usize, len : usize, data : &impl AutoMemory<u8>);
}

pub trait InputDriver : DriverInterrupt {
    // 获取屏幕比例坐标
    fn get_mouse_position(&self)->(f32, f32);
    fn get_key_press(&mut self)->u16;
    fn get_key_release(&mut self)->u16;
    fn get_scroll(&mut self)->u16;
}

// pub trait GPUDriver : DriverInterrupt {
//     fn draw_blend(&mut self, idx : usize, rect : Rect, buffer : *mut Pixel);
//     fn draw_override(&mut self, idx : usize, rect : Rect, buffer : *mut Pixel);
//     fn invalid(&mut self);
// }
