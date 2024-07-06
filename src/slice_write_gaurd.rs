use std::ops::{Deref, DerefMut};

use crate::{LockResult, LockState, SliceReadGaurd};

/// # Slice Write Guard
/// reduces indirection for read gaurds containing slices
/// such as `Box<[T]>` or `Vec<T>`
/// Only points to the slice, so stardard Write Guards may be preferable if you want to mutate a Vec
///  
/// # Examples
/// ```
/// use manual_rwlock::MrwLock;
///
/// let rwlock = MrwLock::new(vec![1, 2, 3]);
/// let mut slice_write = rwlock.try_write_slice().unwrap();
/// slice_write[2] = 4;
/// assert_eq!(*slice_write, [1,2,4])
///
/// ```
pub struct SliceWriteGaurd<'a, T: Sized> {
    pub(super) state: &'a LockState,
    pub(super) data: *mut [T],
}

impl<'a, T> SliceWriteGaurd<'a, T> {
    pub fn to_read(self) -> SliceReadGaurd<'a, T> {
        self.state.to_read();
        SliceReadGaurd {
            state: &self.state,
            data: self.data,
        }
    }

    /// Releases lock without dropping object. This can allow for a write lock to obtained and do some work after which the lock must be reobtained
    ///# Safety
    /// Do not access contents before reobtaining lock with either reobtain or try_reobtain
    /// # Examples
    ///```
    ///  use manual_rwlock::MrwLock;
    ///
    /// let rwlock = MrwLock::new(Vec::from([1,2,3]));
    /// let mut write_rw = rwlock.write_slice().unwrap();
    /// unsafe { write_rw.early_release() };
    /// {
    ///     let mut write2 = rwlock.write_slice().unwrap();
    ///     write2[2] = 4;
    /// }
    /// unsafe { write_rw.reobtain().unwrap() };
    /// write_rw[0] = 4;
    /// assert_eq!(*write_rw, [4,2,4]);
    ///
    ///```
    ///
    pub unsafe fn early_release(&self) {
        self.state.drop_write();
    }

    /// block until lock can be reobtained
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn reobtain(&self) -> LockResult<()> {
        self.state.write()?;
        Ok(())
    }

    /// attempt to reobtain lock, if not possible at this time return `LockError:WouldBlock`
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn try_reobtain(&self) -> LockResult<()> {
        self.state.try_write()?;
        Ok(())
    }
}

impl<'a, T> Drop for SliceWriteGaurd<'a, T> {
    fn drop(&mut self) {
        self.state.drop_write();
    }
}

impl<'a, T> Deref for SliceWriteGaurd<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for SliceWriteGaurd<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}

unsafe impl<'a,T> Send for SliceWriteGaurd<'a, T>{}
unsafe impl<'a, T> Sync for SliceWriteGaurd<'a, T>{}