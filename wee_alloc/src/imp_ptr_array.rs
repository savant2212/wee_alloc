use const_init::ConstInit;
use core::alloc::{AllocErr, Opaque};
#[cfg(feature = "extra_assertions")]
use core::cell::Cell;
use core::ptr::NonNull;
use memory_units::{Bytes, Pages};
use spin::Mutex;

static mut BEGIN: Mutex<usize> = Mutex::new(0);
static mut OFFSET: Mutex<usize> = Mutex::new(0);
static mut SIZE: Mutex<usize> = Mutex::new(0);

pub(crate) unsafe fn alloc_pages(pages: Pages) -> Result<NonNull<Opaque>, AllocErr> {
    let bytes: Bytes = pages.into();
    let mut offset = OFFSET.lock();
    let size = SIZE.lock();
    let begin = BEGIN.lock();
    let end = *offset + bytes.0;
//    panic!("{},{},{},{},{}",bytes.0,*offset,*begin,end,*size);
    if end < (*begin + *size) {
        let ptr = (*begin + *offset) as *mut u8 as *mut Opaque;

        *offset = end;
        NonNull::new(ptr).ok_or_else(||{ 
            AllocErr
        })
    } else {
        Err(AllocErr)
    }
}

#[repr(align(64))]
pub(crate) struct Exclusive<T> {
    inner: Mutex<T>,

    #[cfg(feature = "extra_assertions")]
    in_use: Cell<bool>,
}



impl<T: ConstInit> ConstInit for Exclusive<T> {
    const INIT: Self = Exclusive {
        inner: Mutex::new(T::INIT),

        #[cfg(feature = "extra_assertions")]
        in_use: Cell::new(false),
    };
}

extra_only! {
    fn assert_not_in_use<T>(excl: &Exclusive<T>) {
        assert!(!excl.in_use.get(), "`Exclusive<T>` is not re-entrant");
    }
}

extra_only! {
    fn set_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(true);
    }
}

extra_only! {
    fn set_not_in_use<T>(excl: &Exclusive<T>) {
        excl.in_use.set(false);
    }
}

impl<T> Exclusive<T> {
    /// Get exclusive, mutable access to the inner value.
    ///
    /// # Safety
    ///
    /// It is the callers' responsibility to ensure that `f` does not re-enter
    /// this method for this `Exclusive` instance.
    //
    // XXX: If we don't mark this function inline, then it won't be, and the
    // code size also blows up by about 200 bytes.
    #[inline]
    pub(crate) unsafe fn with_exclusive_access<'a, F, U>(&'a self, f: F) -> U
    where
        for<'x> F: FnOnce(&'x mut T) -> U,
    {
        let mut guard = self.inner.lock();
        assert_not_in_use(self);
        set_in_use(self);
        let result = f(&mut guard);
        set_not_in_use(self);
        result
    }

    /// init statics in runtime
    pub unsafe fn init(&self, heap_start :usize, heap_size :usize) {
        let mut size = SIZE.lock();
        let mut begin = BEGIN.lock();
        *begin = heap_start;
        *size = heap_size;
    }
}
