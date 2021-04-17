//! # 驱动对外接口要求
//! 
//! 2021年4月14日 zg

pub trait Driver {
    /// 处理中断
    fn handler(&mut self);
    /// 通知中断
    fn pending(&mut self);
}

pub trait BlockDriver : Driver {
    fn sync_write(&mut self, offset : usize, len : usize, data : &[u8]);
    fn sync_read(&mut self, offset : usize, len : usize, data : &mut [u8]);
}

pub trait InputDriver : Driver {
    // 获取屏幕比例坐标
    fn get_mouse_position(&self)->(f32, f32);
    fn get_key_press(&mut self)->u16;
    fn get_key_release(&mut self)->u16;
    fn get_scroll(&mut self)->u16;
}

// pub trait GPUDriver : Driver {
//     fn draw_blend(&mut self, idx : usize, rect : Rect, buffer : *mut Pixel);
//     fn draw_override(&mut self, idx : usize, rect : Rect, buffer : *mut Pixel);
//     fn invalid(&mut self);
// }
