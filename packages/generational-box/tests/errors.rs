use std::cell::{Ref, RefMut};

use generational_box::{
    AlreadyBorrowedError, AlreadyBorrowedMutError, AnyStorage, BorrowError, BorrowMutError,
    GenerationalBox, GenerationalRef, GenerationalRefMut, Owner, UnsyncStorage, ValueDroppedError,
};

#[track_caller]
fn read_at_location(
    value: GenerationalBox<i32>,
) -> (
    GenerationalRef<Ref<'static, i32>>,
    &'static std::panic::Location<'static>,
) {
    let location = std::panic::Location::caller();
    let read = value.read();
    (read, location)
}

#[track_caller]
fn write_at_location(
    value: GenerationalBox<i32>,
) -> (
    GenerationalRefMut<RefMut<'static, i32>>,
    &'static std::panic::Location<'static>,
) {
    let location = std::panic::Location::caller();
    let write = value.write();
    (write, location)
}

#[track_caller]
fn create_at_location(
    owner: &Owner,
) -> (GenerationalBox<i32>, &'static std::panic::Location<'static>) {
    let location = std::panic::Location::caller();
    let value = owner.insert(1);
    (value, location)
}

#[test]
fn read_while_writing_error() {
    let owner = UnsyncStorage::owner();
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

#[test]
fn read_after_dropped_error() {
    let owner = UnsyncStorage::owner();
    let (value, location) = create_at_location(&owner);
    drop(owner);
    assert_eq!(
        value.try_read().err(),
        Some(BorrowError::Dropped(ValueDroppedError::new(location)))
    );
}

#[test]
fn write_while_writing_error() {
    let owner = UnsyncStorage::owner();
    let value = owner.insert(1);

    let (write, location) = write_at_location(value);

    assert_eq!(
        value.try_write().err(),
        Some(BorrowMutError::AlreadyBorrowedMut(
            AlreadyBorrowedMutError::new(location)
        ))
    );
    drop(write);
}

#[test]
fn write_while_reading_error() {
    let owner = UnsyncStorage::owner();
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
