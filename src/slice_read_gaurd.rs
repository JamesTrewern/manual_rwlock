use std::ops::Deref;

use crate::{LockResult, LockState, SliceWriteGaurd};

/// # Slice Read Guard
/// reduces indirection for read gaurds containing slices
/// such as `Box<[T]>` or `Vec<T>`
///
/// # Examples
/// ```
/// use manual_rwlock::MrwLock;
///
/// let rwlock = MrwLock::new(vec![1, 2, 3]);
/// let slice_read = rwlock.try_read_slice().unwrap();
/// assert_eq!(*slice_read, [1,2,3])
///
/// ```
pub struct SliceReadGaurd<'a, T: Sized> {
    pub(super) state: &'a LockState,
    pub(super) data: *mut [T],
}

impl<'a, T> SliceReadGaurd<'a, T> {
    pub fn try_to_write(self) -> LockResult<SliceWriteGaurd<'a, T>> {
        self.state.try_to_write()?;
        Ok(SliceWriteGaurd {
            state: &self.state,
            data: self.data,
        })
    }

    pub fn to_write(self) -> LockResult<SliceWriteGaurd<'a, T>> {
        self.state.to_write()?;
        Ok(SliceWriteGaurd {
            state: &self.state,
            data: self.data,
        })
    }

    /// Releases lock without dropping object. This can allow for a write lock to obtained and do some work after which the lock must be reobtained
    ///# Safety
    /// Do not access contents before reobtaining lock with either reobtain or try_reobtain
    /// # Examples
    ///```
    ///  use manual_rwlock::MrwLock;
    ///
    /// let rwlock = MrwLock::new(Vec::from([1,2,3]));
    /// let read_rw = rwlock.read_slice().unwrap();
    /// unsafe { read_rw.early_release() };
    /// {
    ///     let mut write = rwlock.write_slice().unwrap();
    ///     write[2] = 4;
    /// }
    /// unsafe { read_rw.reobtain().unwrap() };
    /// assert_eq!(*read_rw, [1,2,4]);
    ///
    ///```
    ///
    pub unsafe fn early_release(&self) {
        self.state.drop_read();
    }

    /// block until lock can be reobtained
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn reobtain(&self) -> LockResult<()> {
        self.state.read()?;
        Ok(())
    }

    /// attempt to reobtain lock, if not possible at this time return `LockError:WouldBlock`
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn try_reobtain(&self) -> LockResult<()> {
        self.state.try_read()?;
        Ok(())
    }
}

impl<'a, T> Drop for SliceReadGaurd<'a, T> {
    fn drop(&mut self) {
        self.state.drop_read();
    }
}

impl<'a, T> Deref for SliceReadGaurd<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> Clone for SliceReadGaurd<'a, T> {
    fn clone(&self) -> Self {
        self.state.read().unwrap();
        Self { state: self.state, data: self.data }
    }
}