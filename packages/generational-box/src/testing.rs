use core::marker::PhantomData;

#[test]
fn testing() {
    let testing1 = Testing::<_, false>::new(&() as *const _);
    // std::thread::spawn(move || testing1);
}

struct Testing<T, const SYNC: bool> {
    ptr: *mut (),
    phantom: PhantomData<T>,
}

trait CreateNew<T> {
    fn new(data: T) -> Testing<T, true> {
        Testing {
            ptr: &mut (),
            phantom: PhantomData,
        }
    }
}

impl<T> CreateNew<T> for Testing<T, false> {
    fn new(data: T) -> Testing<T, true> {
        Testing {
            ptr: &mut (),
            phantom: PhantomData,
        }
    }
}

impl<T: Sync + Send + 'static> Testing<T, true> {
    pub fn new(data: T) -> Self {
        Testing {
            ptr: &mut (),
            phantom: PhantomData,
        }
    }
}

impl<T: 'static> Testing<T, false> {}

unsafe impl<T: Send + Sync + 'static> Send for Testing<T, true> {}
unsafe impl<T: Send + Sync + 'static> Sync for Testing<T, true> {}
