//! # 设备配置
//! 
//! 2021年3月30日

#![allow(dead_code)]

use crate::input::{InputEvent};
pub const PAGE_SIZE : usize = 4096;

#[derive(Clone, Copy, Default, Debug)]
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

#[derive(Debug)]
pub enum SetupError {
    FeatureFail,
    RingSizeTooSmall,
    Info(&'static str),
}

#[derive(Debug)]
pub enum IoError {
    RequestError,
    Info(&'static str),
}

#[derive(Debug)]
pub enum InterruptOk {
    Null,
    Net,
    Block,
    Graphic,
    Input(InputEvent),
}

#[derive(Debug)]
pub enum InterruptError {
    NoInterrupt,
    Info(&'static str),
}

#[derive(Debug)]
pub enum InputType {
    Event(InputEvent),
    Status(InputEvent),
}

#[derive(Debug)]
pub enum GraphicError {
    InvalidRect(Rect),
    BufferTooSmall(usize),
}

#[derive(Copy, Clone)]
pub enum DeviceType {
    Unknown = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    Entropy = 4,
    Gpu = 16,
    Input = 18,
    Memory = 24,
}

impl DeviceType {
    pub fn from(num : usize)->Self {
        match num {
            2 => {DeviceType::Block}
            16 => {DeviceType::Gpu}
            18 => {DeviceType::Input}
            1 => {DeviceType::Network}
            _ => {DeviceType::Unknown}
        }
    }
}
