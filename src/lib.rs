#![no_std]
mod require;
mod queue;
mod header;
mod config;
mod block;
mod gpu;
mod input;
mod pool;
mod net;

use config::{GraphicError, IoError};
pub use config::{InterruptError, InterruptOk, DeviceType};
pub use header::VirtHeader;
pub use queue::VirtQueue;
pub use block::Block;
pub use gpu::GPU;
pub use net::Net;
pub use input::{InputDevice, InputEvent};
pub use config::{
    Pixel,
    Rect,
};
pub use require::*;

pub type InterruptResult = Result<InterruptOk, InterruptError>;
pub type IoResult = Result<(), IoError>;
pub type GraphicResult = Result<(), GraphicError>;
