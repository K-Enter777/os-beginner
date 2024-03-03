use core::{
    marker::PhantomData,
    mem::{size_of, transmute},
};

use crate::{
    error::{self, Result},
    make_error,
};

pub(crate) struct ArrayQueue<'a, T> {
    data: &'a mut [T],
    read_pos: usize,
    write_pos: usize,
    count: usize,
    capacity: usize,
    marker: PhantomData<T>,
}

impl<'a, T> ArrayQueue<'a, T> {
    #![allow(unused)]

    pub(crate) fn new(buf: &'a mut [u8]) -> Self {
        let capacity = buf.len() / size_of::<T>();
        let buf: &mut [T] = unsafe { transmute(buf) };
        Self {
            data: buf,
            read_pos: 0,
            write_pos: 0,
            count: 0,
            capacity,
            marker: PhantomData,
        }
    }

    pub(crate) fn push(&mut self, value: T) -> Result<()> {
        if self.count == self.capacity {
            return Err(make_error!(error::Code::Full));
        }

        self.data[self.write_pos] = value;

        self.count += 1;

        self.write_pos = if self.write_pos + 1 == self.capacity {
            0
        } else {
            self.write_pos + 1
        };

        Ok(())
    }

    pub(crate) fn front(&self) -> Option<&T> {
        if self.count == 0 {
            None
        } else {
            Some(&self.data[self.read_pos])
        }
    }

    pub(crate) fn pop(&mut self) -> Result<()> {
        if self.count == 0 {
            return Err(make_error!(error::Code::Empty));
        }

        self.count -= 1;

        self.read_pos = if self.read_pos + 1 == self.capacity {
            0
        } else {
            self.read_pos + 1
        };

        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.count
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }
}
