use crate::{write_gaurd::WriteGaurd, LockError, LockResult, LockState};
use std::ops::Deref;

pub struct ReadGaurd<'a, T: Sized> {
    pub(super) state: &'a LockState,
    pub(super) data: *mut T,
}

impl<'a, T> ReadGaurd<'a, T> {
    pub fn try_to_write(self) -> LockResult<WriteGaurd<'a, T>> {
        self.state.try_to_write()?;
        Ok(WriteGaurd {
            state: &self.state,
            data: self.data,
        })
    }

    pub fn to_write(self) -> LockResult<WriteGaurd<'a, T>> {
        self.state.to_write()?;
        Ok(WriteGaurd {
            state: &self.state,
            data: self.data,
        })
    }

    /// Releases lock without dropping object. This can allow for a write lock to obtained and do some work after which the lock must be reobtained
    ///# Safety
    /// Do not access contents before reobtaining lock with either reobtain or try_reobtain
    /// # Examples
    ///```
    ///  use manual_rwlock::MrwLock
    ///
    /// let rwlock = MrwLock::new(5);
    /// let read_rw = rwlock.read().unwrap();
    /// unsafe { read_rw.early_release() };
    /// {
    ///     let mut write = rwlock.write().unwrap();
    ///     *write += 5;
    /// }
    /// unsafe { read_rw.reobtain().unwrap() };
    /// assert_eq!(*read_rw, 10);
    ///
    ///```
    ///
    pub unsafe fn early_release(&self) {
        self.state.drop_read();
    }

    /// block until lock can be reobtained
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn reobtain(&self) -> Result<(), LockError> {
        self.state.read()?;
        Ok(())
    }

    /// attempt to reobtain lock, if not possible at this time return `LockError:WouldBlock`
    /// # Safety
    /// do not use unless early release has been called. Only call at most once after each early release
    pub unsafe fn try_reobtain(&self) -> Result<(), LockError> {
        self.state.try_read()?;
        Ok(())
    }
}

impl<'a, T> Drop for ReadGaurd<'a, T> {
    fn drop(&mut self) {
        self.state.drop_read();
    }
}

impl<'a, T> Deref for ReadGaurd<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
    }
}
