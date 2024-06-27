//! # Manual RwLock
//! A library implementing An RW lock with more manual control 
//! Sorry for poor documentation, I will update this later.
//! <br> New Read Write Lock struct: [MrwLock]
//! # Examples
//! # Convert between Guards
//! # Release and Reobtain locks
//! # Use Locking Directly
//! a link to 
//! [LockState]
//!
//!     
//! 
mod read_gaurd;
mod slice_read_gaurd;
mod slice_write_gaurd;
#[cfg(test)]
mod tests;
mod write_gaurd;

use atomic_wait::wait;
use std::{
    cell::UnsafeCell,
    sync::atomic::{
        AtomicBool, AtomicU32,
        Ordering::{Acquire, Relaxed},
    },
};

pub use read_gaurd::ReadGaurd;
pub use slice_read_gaurd::SliceReadGaurd;
pub use slice_write_gaurd::SliceWriteGaurd;
pub use write_gaurd::WriteGaurd;

#[derive(Debug)]
pub enum LockError {
    TooManyReaders,
    WouldBlock,
    Poisoned,
}
pub type LockResult<Gaurd> = Result<Gaurd, LockError>;

/// State that manages control flow for [MrwLock] and Guards: [SliceReadGaurd], [ReadGaurd] ,[SliceWriteGaurd], [WriteGaurd]
/// Passing by reference allows interior mutability
pub struct LockState {
    state: AtomicU32,
    poisoned: AtomicBool,
}

impl LockState {
    ///Creates new lock state
    pub fn new() -> LockState {
        LockState {
            state: 0.into(),
            poisoned: false.into(),
        }
    }

    ///Increment number of readers. If there is a write lock block thread until read lock can be obtained
    pub fn read(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        let mut s = self.state.load(Relaxed);
        loop {
            if s == u32::MAX {
                wait(&self.state, u32::MAX);
                s = self.state.load(Relaxed);
            } else if s == u32::MAX - 1 {
                return Err(LockError::TooManyReaders);
            } else {
                match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                    Ok(_) => return Ok(()),
                    Err(e) => s = e,
                }
            }
        }
    }

    ///Increment number of readers. If there is a write lock return [LockError::WouldBlock]
    pub fn try_read(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        let mut s = self.state.load(Relaxed);
        loop {
            if s < u32::MAX {
                if s == u32::MAX - 1 {
                    return Err(LockError::TooManyReaders);
                }
                match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                    Ok(_) => return Ok(()),

                    Err(e) => s = e,
                }
            } else {
                return Err(LockError::WouldBlock);
            }
        }
    }

    ///Attempt write lock. If there is another lock block thread until the write lock can be obtained
    pub fn write(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        while let Err(s) = self.state.compare_exchange(0, u32::MAX, Acquire, Relaxed) {
            // Wait while already locked.
            wait(&self.state, s);
        }
        self.poisoned.store(true, Relaxed);
        Ok(())
    }

    ///Attempt write lock. If there is another lock return [LockError::WouldBlock]
    pub fn try_write(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        let s = self.state.load(Relaxed);
        if s == 0 {
            match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                Ok(_) => {
                    self.poisoned.store(true, Relaxed);
                    return Ok(());
                }
                Err(e) => return Err(LockError::WouldBlock),
            }
        } else {
            return Err(LockError::WouldBlock);
        }
    }

    ///Convert a read lock into a write lock, if there is another lock block thread until write lock can be obtained
    pub fn to_write(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        while let Err(s) = self.state.compare_exchange(1, u32::MAX, Acquire, Relaxed) {
            // Wait while already locked.
            wait(&self.state, s);
        }
        self.poisoned.store(true, Relaxed);
        Ok(())
    }

    ///Attempt to convert a read lock into a write lock, if there is another lock return [LockError::WouldBlock]
    pub fn try_to_write(&self) -> LockResult<()> {
        if self.poisoned.load(Relaxed) {
            return Err(LockError::Poisoned);
        }
        let s = self.state.load(Relaxed);
        if s == 1 {
            match self
                .state
                .compare_exchange_weak(s, u32::MAX, Acquire, Relaxed)
            {
                Ok(_) => Ok(()),
                Err(e) => Err(LockError::WouldBlock),
            }
        } else {
            Err(LockError::WouldBlock)
        }
    }

    ///Convert write lock to read lock
    pub fn to_read(&self) {
        self.state.store(1, Relaxed);
        self.poisoned.store(false, Relaxed);
    }

    ///Drop read lock. Decrements the total nubmer of readers
    pub fn drop_read(&self) {
        self.poisoned.store(false, Relaxed);
        self.state.fetch_sub(1, Relaxed);
    }

    ///Drop write lock. Sets number of readers to 0;
    pub fn drop_write(&self) {
        self.state.store(0, Relaxed);
    }
}

pub struct MrwLock<T: Sized> {
    state: LockState,
    data: UnsafeCell<T>,
}

impl<T> MrwLock<T> {
    pub fn new(data: T) -> MrwLock<T> {
        MrwLock {
            state: LockState::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn try_read(&self) -> LockResult<ReadGaurd<T>> {
        self.state.try_read()?;
        Ok(ReadGaurd {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn read(&self) -> LockResult<ReadGaurd<T>> {
        self.state.read()?;
        Ok(ReadGaurd {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn try_write(&self) -> LockResult<WriteGaurd<T>> {
        self.state.try_write();
        Ok(WriteGaurd {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn write(&self) -> LockResult<WriteGaurd<T>> {
        self.state.write()?;
        Ok(WriteGaurd {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn try_read_slice<U: Sized>(&self) -> LockResult<SliceReadGaurd<U>>
    where
        T: AsMut<[U]>,
    {
        self.state.try_read()?;
        Ok(SliceReadGaurd {
            state: &self.state,
            data: unsafe { (*self.data.get()).as_mut() } as *mut [U],
        })
    }

    pub fn read_slice<U: Sized>(&self) -> LockResult<SliceReadGaurd<U>>
    where
        T: AsMut<[U]>,
    {
        self.state.read()?;
        Ok(SliceReadGaurd {
            state: &self.state,
            data: unsafe { (*self.data.get()).as_mut() } as *mut [U],
        })
    }

    pub fn try_write_slice<U>(&self) -> LockResult<SliceWriteGaurd<U>>
    where
        T: AsMut<[U]>,
    {
        self.state.try_write()?;
        Ok(SliceWriteGaurd {
            state: &self.state,
            data: unsafe { (*self.data.get()).as_mut() } as *mut [U],
        })
    }

    pub fn write_slice<U>(&self) -> LockResult<SliceWriteGaurd<U>>
    where
        T: AsMut<[U]>,
    {
        self.state.write()?;
        Ok(SliceWriteGaurd {
            state: &self.state,
            data: unsafe { (*self.data.get()).as_mut() } as *mut [U],
        })
    }
}
