use generational_box::{
    AlreadyBorrowedError, AlreadyBorrowedMutError, BorrowError, BorrowMutError, GenerationalBox,
    Owner, Storage, SyncStorage, UnsyncStorage, ValueDroppedError,
};

#[track_caller]
fn read_at_location<S: Storage<i32>>(
    value: GenerationalBox<i32, S>,
) -> (S::Ref<'static, i32>, &'static std::panic::Location<'static>) {
    let location = std::panic::Location::caller();
    let read = value.read();
    (read, location)
}

#[track_caller]
fn write_at_location<S: Storage<i32>>(
    value: GenerationalBox<i32, S>,
) -> (S::Mut<'static, i32>, &'static std::panic::Location<'static>) {
    let location = std::panic::Location::caller();
    let write = value.write();
    (write, location)
}

#[track_caller]
fn create_at_location<S: Storage<i32>>(
    owner: &Owner<S>,
) -> (
    GenerationalBox<i32, S>,
    &'static std::panic::Location<'static>,
) {
    let location = std::panic::Location::caller();
    let value = owner.insert(1);
    (value, location)
}

#[test]
fn read_while_writing_error() {
    fn read_while_writing_error_test<S: Storage<i32>>() {
        let owner = S::owner();
        let value = owner.insert(1);

        let (write, location) = write_at_location(value);

        assert_eq!(
            value.try_read().err(),
            Some(BorrowError::AlreadyBorrowedMut(
                AlreadyBorrowedMutError::new(location)
            ))
        );
        drop(write);
    }

    // For sync storage this will deadlock
    read_while_writing_error_test::<UnsyncStorage>();
}

#[test]
fn read_after_dropped_error() {
    fn read_after_dropped_error_test<S: Storage<i32>>() {
        let owner = S::owner();
        let (value, location) = create_at_location(&owner);
        drop(owner);
        assert_eq!(
            value.try_read().err(),
            Some(BorrowError::Dropped(ValueDroppedError::new(location)))
        );
    }

    read_after_dropped_error_test::<UnsyncStorage>();
    read_after_dropped_error_test::<SyncStorage>();
}

#[test]
fn write_while_writing_error() {
    fn write_while_writing_error_test<S: Storage<i32>>() {
        let owner = S::owner();
        let value = owner.insert(1);

        #[allow(unused)]
        let (write, location) = write_at_location(value);

        let write_result = value.try_write();
        assert!(write_result.is_err());
        #[cfg(any(debug_assertions, feature = "debug_borrows"))]
        assert_eq!(
            write_result.err(),
            Some(BorrowMutError::AlreadyBorrowedMut(
                AlreadyBorrowedMutError::new(location)
            ))
        );

        drop(write);
    }

    // For sync storage this will deadlock
    write_while_writing_error_test::<UnsyncStorage>();
}

#[test]
fn write_while_reading_error() {
    fn write_while_reading_error_test<S: Storage<i32>>() {
        let owner = S::owner();
        let value = owner.insert(1);

        let (read, location) = read_at_location(value);

        assert_eq!(
            value.try_write().err(),
            Some(BorrowMutError::AlreadyBorrowed(AlreadyBorrowedError::new(
                vec![location]
            )))
        );

        drop(read);
    }

    // For sync storage this will deadlock
    write_while_reading_error_test::<UnsyncStorage>();
}
