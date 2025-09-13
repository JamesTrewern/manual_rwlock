//! # Manual RwLock
//! A library implementing An RW lock with more manual control
//! Sorry for poor documentation, I will update this later.
//! <br> New read write lock struct: [MrwLock]
//! # Examples
//! # Convert between Guards
//! ```
//! use manual_rwlock::MrwLock;
//! let mrw_lock = MrwLock::new(10);
//! let read = mrw_lock.read().unwrap();
//! let mut write = read.to_write().unwrap();
//! *write = 5;
//! let read = write.to_read();
//! assert_eq!(*read, 5)
//! ```
//! # Release and Reobtain locks
//! ```
//! use manual_rwlock::MrwLock;
//! let mrw_lock = MrwLock::new(10);
//! let read = mrw_lock.read().unwrap();
//! unsafe {read.early_release();}
//! {
//!     let mut write = mrw_lock.write().unwrap();
//!     *write = 5;
//! }
//! unsafe {read.reobtain();}
//! assert_eq!(*read, 5)
//! ```
//! # Clone [ReadGaurd]
//! ```
//!  use manual_rwlock::MrwLock;
//!
//! let rwlock = MrwLock::new(5);
//! let read = rwlock.read().unwrap();
//! let read2 = read.clone();
//! assert_eq!(*read2, 5);
//! ```
//! # Use Locking Directly
//! [LockState]
//!
//!     
//!
mod read_guard;
mod slice_read_guard;
mod slice_write_guard;
#[cfg(test)]
mod tests;
mod write_guard;

use atomic_wait::wait;
use std::{
    borrow::BorrowMut,
    cell::UnsafeCell,
    sync::atomic::{
        AtomicBool, AtomicU32,
        Ordering::{Acquire, Relaxed},
    },
    thread,
};

pub use read_guard::ReadGuard;
pub use slice_read_guard::SliceReadGuard;
pub use slice_write_guard::SliceWriteGuard;
pub use write_guard::WriteGuard;

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
    pub const fn new() -> LockState {
        LockState {
            state: AtomicU32::new(0),
            poisoned: AtomicBool::new(false),
        }
    }

    ///Increment number of readers. If there is a write lock block thread until read lock can be obtained
    pub fn read(&self) -> LockResult<()> {
        let mut s = self.state.load(Relaxed);
        loop {
            if s == u32::MAX {
                wait(&self.state, u32::MAX);
                s = self.state.load(Relaxed);
            } else if s == u32::MAX - 1 {
                return Err(LockError::TooManyReaders);
            } else {
                match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                    Ok(_) => {
                        if self.poisoned.load(Relaxed) {
                            return Err(LockError::Poisoned);
                        } else {
                            return Ok(());
                        }
                    }
                    Err(e) => s = e,
                }
            }
        }
    }

    ///Increment number of readers. If there is a write lock return [LockError::WouldBlock]
    pub fn try_read(&self) -> LockResult<()> {
        let mut s = self.state.load(Relaxed);
        loop {
            if s < u32::MAX {
                if s == u32::MAX - 1 {
                    return Err(LockError::TooManyReaders);
                }
                match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                    Ok(_) => {
                        if self.poisoned.load(Relaxed) {
                            return Err(LockError::Poisoned);
                        } else {
                            return Ok(());
                        }
                    }

                    Err(e) => s = e,
                }
            } else {
                return Err(LockError::WouldBlock);
            }
        }
    }

    ///Attempt write lock. If there is another lock block thread until the write lock can be obtained
    pub fn write(&self) -> LockResult<()> {
        while let Err(s) = self.state.compare_exchange(0, u32::MAX, Acquire, Relaxed) {
            // Wait while already locked.
            wait(&self.state, s);
        }
        if self.poisoned.load(Relaxed) {
            Err(LockError::Poisoned)
        } else {
            Ok(())
        }
    }

    ///Attempt write lock. If there is another lock return [LockError::WouldBlock]
    pub fn try_write(&self) -> LockResult<()> {
        let s = self.state.load(Relaxed);
        if s == 0 {
            match self.state.compare_exchange_weak(s, s + 1, Acquire, Relaxed) {
                Ok(_) => {
                    if self.poisoned.load(Relaxed) {
                        Err(LockError::Poisoned)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(LockError::WouldBlock),
            }
        } else {
            Err(LockError::WouldBlock)
        }
    }

    ///Convert a read lock into a write lock, if there is another lock block thread until write lock can be obtained
    pub fn to_write(&self) -> LockResult<()> {
        while let Err(s) = self.state.compare_exchange(1, u32::MAX, Acquire, Relaxed) {
            // Wait while already locked.
            wait(&self.state, s);
        }
        if self.poisoned.load(Relaxed) {
            Err(LockError::Poisoned)
        } else {
            Ok(())
        }
    }

    ///Attempt to convert a read lock into a write lock, if there is another lock return [LockError::WouldBlock]
    pub fn try_to_write(&self) -> LockResult<()> {
        let s = self.state.load(Relaxed);
        if s == 1 {
            match self
                .state
                .compare_exchange_weak(s, u32::MAX, Acquire, Relaxed)
            {
                Ok(_) => {
                    if self.poisoned.load(Relaxed) {
                        Err(LockError::Poisoned)
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(LockError::WouldBlock),
            }
        } else {
            Err(LockError::WouldBlock)
        }
    }

    ///Convert write lock to read lock
    pub fn to_read(&self) {
        self.state.store(1, Relaxed);
    }

    ///Drop read lock. Decrements the total nubmer of readers
    pub fn drop_read(&self) {
        self.state.fetch_sub(1, Relaxed);
    }

    ///Drop write lock. Sets number of readers to 0;
    pub fn drop_write(&self) {
        if thread::panicking() {
            self.poisoned.store(true, Relaxed);
        }
        self.state.store(0, Relaxed);
    }
}

pub struct MrwLock<T: Sized> {
    state: LockState,
    data: UnsafeCell<T>,
}

impl<T> MrwLock<T> {
    pub const fn new(data: T) -> MrwLock<T> {
        MrwLock {
            state: LockState::new(),
            data: UnsafeCell::new(data),
        }
    }

    pub fn try_read(&self) -> LockResult<ReadGuard<T>> {
        self.state.try_read()?;
        Ok(ReadGuard {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn read(&self) -> LockResult<ReadGuard<T>> {
        self.state.read()?;
        Ok(ReadGuard {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn try_write(&self) -> LockResult<WriteGuard<T>> {
        self.state.try_write()?;
        Ok(WriteGuard {
            state: &self.state,
            data: self.data.get(),
        })
    }

    pub fn write(&self) -> LockResult<WriteGuard<T>> {
        self.state.write()?;
        Ok(WriteGuard {
            state: &self.state,
            data: self.data.get(),
        })
    }
}

impl<T> MrwLock<T>
{
    pub fn try_read_slice<U>(&self) -> LockResult<SliceReadGuard<U>>
    where
        T: BorrowMut<[U]>,
    {
        self.state.try_read()?;
        Ok(SliceReadGuard {
            state: &self.state,
            data: unsafe { (*self.data.get()).borrow_mut() } as *mut [U],
        })
    }

    pub fn read_slice<U>(&self) -> LockResult<SliceReadGuard<U>>
    where
        T: BorrowMut<[U]>,
    {
        self.state.read()?;
        Ok(SliceReadGuard {
            state: &self.state,
            data: unsafe { (*self.data.get()).borrow_mut() } as *mut [U],
        })
    }

    pub fn try_write_slice<U>(&self) -> LockResult<SliceWriteGuard<U>>
    where
        T: BorrowMut<[U]>,
    {
        self.state.try_write()?;
        Ok(SliceWriteGuard {
            state: &self.state,
            data: unsafe { (*self.data.get()).borrow_mut() } as *mut [U],
        })
    }

    pub fn write_slice<U>(&self) -> LockResult<SliceWriteGuard<U>>
    where
        T: BorrowMut<[U]>,
    {
        self.state.write()?;
        Ok(SliceWriteGuard {
            state: &self.state,
            data: unsafe { (*self.data.get()).borrow_mut() } as *mut [U],
        })
    }
}


unsafe impl<T> Send for MrwLock<T>{}
unsafe impl<T> Sync for MrwLock<T>{}
