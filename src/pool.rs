//! # 命令池
//! 命令的内存从此处索取，不需要依赖内核内存管理
//!
//! 2021年4月18日 zg

use crate::queue::VIRTIO_RING_SIZE;

pub struct Pool<T:Clone + Default> {
    queue : [T;VIRTIO_RING_SIZE],
}

impl<T:Clone + Copy + Default> Default for Pool<T> {
    fn default() -> Self {
        Self{
            queue : [T::default();VIRTIO_RING_SIZE],
        }
    }
}

impl<T:Clone + Copy + Default> Pool<T> {
    pub fn get(&mut self, idx : usize)->&mut T {
        &mut self.queue[idx]
    }

    pub fn replace_ref(&mut self, idx : usize, v : T)->&mut T {
        self.queue[idx] = v;
        &mut self.queue[idx]
    }

    pub fn replace_u64(&mut self, idx : usize, v : T)->u64 {
        self.queue[idx] = v;
        &mut self.queue[idx] as *mut T as u64
    }
}

