//! # 驱动对外接口要求
//! 
//! 2021年4月14日 zg

use crate::{GraphicResult, InterruptResult, IoResult, Rect, config::{Pixel}};

pub trait Driver {
    /// 处理中断
    fn handler(&mut self)->InterruptResult;
    /// 通知中断
    fn pending(&mut self)->InterruptResult;
}

pub trait BlockDriver : Driver {
    fn sync_write(&mut self, offset : usize, len : usize, data : &[u8])->IoResult;
    fn sync_read(&mut self, offset : usize, len : usize, data : &mut [u8])->IoResult;
}

pub trait GraphicDriver : Driver {
    fn draw_blend(&mut self, rect : Rect, buffer : &[Pixel])->GraphicResult;
    fn draw_override(&mut self, rect : Rect, buffer : &[Pixel])->GraphicResult;
    fn refresh(&mut self);
}

pub trait NetDriver : Driver {
    fn send(&mut self, data : &[u8]);
    fn mac(&self)->usize;
}
