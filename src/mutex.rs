use core::cell::UnsafeCell;
use core::default::Default;
use core::fmt::Debug;
use core::fmt::Formatter;
// use core::marker::Sync;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

use psp2_sys::kernel::threadmgr::sceKernelCreateMutex;
use psp2_sys::kernel::threadmgr::sceKernelDelayThread;
use psp2_sys::kernel::threadmgr::sceKernelDeleteMutex;
use psp2_sys::kernel::threadmgr::sceKernelLockMutex;
use psp2_sys::kernel::threadmgr::sceKernelTryLockMutex;
use psp2_sys::kernel::threadmgr::sceKernelUnlockMutex;
use psp2_sys::types::SceUID;

/// This struct provides MUTual EXclusion using the Kernel API.
pub struct Mutex<T: ?Sized> {
    lock: SceUID,        // the kernel lock
    init: AtomicBool,    // false until the lock initialization has started
    done: AtomicBool,    // false until the lock initialization has finished
    data: UnsafeCell<T>, // the synced data
}

/// A guard to which the protected data can be accessed
///
/// When the guard falls out of scope it will release the lock.
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: SceUID,
    data: &'a UnsafeCell<T>,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new lock wrapping the supplied data.
    ///
    /// May be used statically:
    ///
    /// ```rust,ignore
    /// #![feature(const_fn)]
    /// use vitalloc;
    ///
    /// static MUTEX: vitalloc::Mutex<()> = vitalloc::Mutex::new(());
    ///
    /// fn demo() {
    ///     let lock = MUTEX.lock();
    ///     // do something with lock
    ///     drop(lock);
    /// }
    /// ```
    pub const fn new(user_data: T) -> Mutex<T> {
        Mutex {
            lock: 0,
            init: AtomicBool::new(false),
            done: AtomicBool::new(false),
            data: UnsafeCell::new(user_data),
        }
    }

    /// Consumes this mutex, returning the underlying data.
    pub fn into_inner(self) -> T {
        // We know statically that there are no outstanding references to
        // `self` so there's no need to lock the inner mutex.
        //
        // To get the inner value, we'd like to call `data.into_inner()`,
        // but because `Mutex` impl-s `Drop`, we can't move out of it, so
        // we'll have to destructure it manually instead.
        let data = unsafe {
            let Mutex { ref data, .. } = self;
            ::core::ptr::read(data)
        };

        data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Check the lock was properly initializated and return it.
    fn init_lock(&self) -> SceUID {
        if self.init.compare_and_swap(false, true, Ordering::Acquire) == false {
            // if the lock initialization was not started: start it !
            // - name the mutex according to its memory location
            let ptr: usize = self as *const Self as *const usize as usize;
            let mut name = *b"__rust_mutex_0x00000000\0";
            ::utils::write_hex(ptr, &mut name[15..23]);
            // - initialize the mutex
            unsafe { sceKernelCreateMutex(&name as *const u8, 0, 0, ::core::ptr::null_mut()) };
            self.done.store(true, Ordering::Relaxed);
        } else {
            // wait for the lock initialization to be over.
            while !self.done.load(Ordering::Relaxed) {
                unsafe { sceKernelDelayThread(1000) };
            }
        }
        self.lock
    }

    /// Locks the spinlock and returns a guard.
    ///
    /// The returned value may be dereferenced for data access
    /// and the lock will be dropped when the guard falls out of scope.
    ///
    /// ```rust,ignore
    /// let mylock = vitalloc::Mutex::new(0);
    /// {
    ///     let mut data = mylock.lock();
    ///     // The lock is now locked and the data can be accessed
    ///     *data += 1;
    ///     // The lock is implicitly dropped
    /// }
    ///
    /// ```
    pub fn lock(&self) -> MutexGuard<T> {
        let lock = self.init_lock();
        unsafe { sceKernelLockMutex(lock, 1, ::core::ptr::null_mut()) };
        MutexGuard {
            lock: lock,
            data: &self.data,
        }
    }

    /// Tries to lock the mutex. If it is already locked, it will return None. Otherwise it returns
    /// a guard within Some.
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        let lock = self.init_lock();
        if unsafe { sceKernelTryLockMutex(lock, 1) } >= 0 {
            Some(MutexGuard {
                lock: lock,
                data: &self.data,
            })
        } else {
            None
        }
    }
}

impl<T: ?Sized + Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut Formatter) -> ::core::fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: {:?} }}", &*guard),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

impl<'a, T: ?Sized> ::core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref<'b>(&'b self) -> &'b T {
        unsafe { &(*self.data.get()) }
    }
}

impl<'a, T: ?Sized> ::core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut T {
        unsafe { &mut (*self.data.get()) }
    }
}

impl<'a, T: ?Sized> ::core::ops::Drop for MutexGuard<'a, T> {
    /// The dropping of the MutexGuard will release the lock it was created from.
    fn drop(&mut self) {
        unsafe { sceKernelUnlockMutex(self.lock, 1) };
    }
}

impl<T: ?Sized> ::core::ops::Drop for Mutex<T> {
    fn drop(&mut self) {
        unsafe { sceKernelDeleteMutex(self.lock) };
    }
}

// #[cfg(test)]
// mod tests {
//     use std::prelude::v1::*;
//
//     use std::sync::mpsc::channel;
//     use std::sync::Arc;
//     use std::sync::atomic::{AtomicUsize, Ordering};
//     use std::thread;
//
//     use super::*;
//
//     #[derive(Eq, PartialEq, Debug)]
//     struct NonCopy(i32);
//
//     #[test]
//     fn smoke() {
//         let m = Mutex::new(());
//         drop(m.lock());
//         drop(m.lock());
//     }
//
//     #[test]
//     #[cfg(feature = "const_fn")]
//     fn lots_and_lots() {
//         static M: Mutex<()>  = Mutex::new(());
//         static mut CNT: u32 = 0;
//         const J: u32 = 1000;
//         const K: u32 = 3;
//
//         fn inc() {
//             for _ in 0..J {
//                 unsafe {
//                     let _g = M.lock();
//                     CNT += 1;
//                 }
//             }
//         }
//
//         let (tx, rx) = channel();
//         for _ in 0..K {
//             let tx2 = tx.clone();
//             thread::spawn(move|| { inc(); tx2.send(()).unwrap(); });
//             let tx2 = tx.clone();
//             thread::spawn(move|| { inc(); tx2.send(()).unwrap(); });
//         }
//
//         drop(tx);
//         for _ in 0..2 * K {
//             rx.recv().unwrap();
//         }
//         assert_eq!(unsafe {CNT}, J * K * 2);
//     }
//
//     #[test]
//     fn try_lock() {
//         let mutex = Mutex::new(42);
//
//         // First lock succeeds
//         let a = mutex.try_lock();
//         assert_eq!(a.as_ref().map(|r| **r), Some(42));
//
//         // Additional lock failes
//         let b = mutex.try_lock();
//         assert!(b.is_none());
//
//         // After dropping lock, it succeeds again
//         ::core::mem::drop(a);
//         let c = mutex.try_lock();
//         assert_eq!(c.as_ref().map(|r| **r), Some(42));
//     }
//
//     #[test]
//     fn test_into_inner() {
//         let m = Mutex::new(NonCopy(10));
//         assert_eq!(m.into_inner(), NonCopy(10));
//     }
//
//     #[test]
//     fn test_into_inner_drop() {
//         struct Foo(Arc<AtomicUsize>);
//         impl Drop for Foo {
//             fn drop(&mut self) {
//                 self.0.fetch_add(1, Ordering::SeqCst);
//             }
//         }
//         let num_drops = Arc::new(AtomicUsize::new(0));
//         let m = Mutex::new(Foo(num_drops.clone()));
//         assert_eq!(num_drops.load(Ordering::SeqCst), 0);
//         {
//             let _inner = m.into_inner();
//             assert_eq!(num_drops.load(Ordering::SeqCst), 0);
//         }
//         assert_eq!(num_drops.load(Ordering::SeqCst), 1);
//     }
//
//     #[test]
//     fn test_mutex_arc_nested() {
//         // Tests nested mutexes and access
//         // to underlying data.
//         let arc = Arc::new(Mutex::new(1));
//         let arc2 = Arc::new(Mutex::new(arc));
//         let (tx, rx) = channel();
//         let _t = thread::spawn(move|| {
//             let lock = arc2.lock();
//             let lock2 = lock.lock();
//             assert_eq!(*lock2, 1);
//             tx.send(()).unwrap();
//         });
//         rx.recv().unwrap();
//     }
//
//     #[test]
//     fn test_mutex_arc_access_in_unwind() {
//         let arc = Arc::new(Mutex::new(1));
//         let arc2 = arc.clone();
//         let _ = thread::spawn(move|| -> () {
//             struct Unwinder {
//                 i: Arc<Mutex<i32>>,
//             }
//             impl Drop for Unwinder {
//                 fn drop(&mut self) {
//                     *self.i.lock() += 1;
//                 }
//             }
//             let _u = Unwinder { i: arc2 };
//             panic!();
//         }).join();
//         let lock = arc.lock();
//         assert_eq!(*lock, 2);
//     }
//
//     #[test]
//     fn test_mutex_unsized() {
//         let mutex: &Mutex<[i32]> = &Mutex::new([1, 2, 3]);
//         {
//             let b = &mut *mutex.lock();
//             b[0] = 4;
//             b[2] = 5;
//         }
//         let comp: &[i32] = &[4, 2, 5];
//         assert_eq!(&*mutex.lock(), comp);
//     }
//
//     #[test]
//     fn test_mutex_force_lock() {
//         let lock = Mutex::new(());
//         ::std::mem::forget(lock.lock());
//         unsafe {
//             lock.force_unlock();
//         }
//         assert!(lock.try_lock().is_some());
//     }
// }
