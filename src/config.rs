//! # 设备配置
//! 
//! 2021年3月30日

#![allow(dead_code)]
pub const PAGE_SIZE : usize = 4096;

#[derive(Clone, Copy, Default)]
pub struct Rect {
    pub x1 : u32,
    pub y1 : u32,
    pub x2 : u32,
    pub y2 : u32,
}

#[derive(Clone, Copy)]
pub struct Pixel {
    pub r : u8,
    pub g : u8,
    pub b : u8,
    pub a : u8,
}

impl Pixel{
    pub fn new(r:u8, g:u8, b:u8, a:u8)->Self {
        Self {
            r : r,
            g : g,
            b : b,
            a : a,
        }
    }
    pub const fn red()->Self{
        Pixel{
            r:255,
            g:0,
            b:0,
            a:255,
        }
    }
    pub const fn green()->Self{
        Self{
            r:0,
            g:255,
            b:0,
            a:255
        }
    }
    pub const fn blue()->Self{
        Self{
            r:0,
            g:0,
            b:255,
            a:255
        }
    }
    pub const fn yellow()->Self{
        Self{
            r:255,
            g:255,
            b:0,
            a:255
        }
    }
    pub const fn grey()->Self{
        Self{
            r:55,
            g:55,
            b:55,
            a:255,
        }
    }
    pub const fn white()->Self{
        Self{
            r:255,
            g:255,
            b:255,
            a:255
        }
    }
    pub const fn black()->Self{
        Self{
            r : 0,
            g : 0,
            b : 0,
            a : 255,
        }
    }
    pub const fn shallow_grey()->Self{
        Self{
            r:122,
            g:122,
            b:122,
            a:255,
        }
    }
}
