use std::ops::{Deref, DerefMut};

use crate::{LockResult, LockState, ReadGaurd};

pub struct WriteGaurd<'a, T: Sized> {
    pub(super) state: &'a LockState,
    pub(super) data: *mut T,
}

impl<'a, T> WriteGaurd<'a, T> {
    pub fn to_read(self) -> ReadGaurd<'a, T> {
        self.state.to_read();
        ReadGaurd {
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
    /// let mut write_rw = rwlock.write().unwrap();
    /// unsafe { write_rw.early_release() };
    /// {
    ///     let mut write2 = rwlock.write().unwrap();
    ///     write2.push(4);
    /// }
    /// unsafe { write_rw.reobtain().unwrap() };
    /// write_rw.push(5);
    /// assert_eq!(*write_rw, [1,2,3,4,5]);
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

impl<'a, T> Drop for WriteGaurd<'a, T> {
    fn drop(&mut self) {
        self.state.drop_write();
    }
}

impl<'a, T> Deref for WriteGaurd<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}

impl<'a, T> DerefMut for WriteGaurd<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data }
    }
}
