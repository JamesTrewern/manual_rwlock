use crate::MrwLock;

#[test]
fn early_release() {
    let rwlock = MrwLock::new(5);
    let read_rw = rwlock.read().unwrap();
    unsafe { read_rw.early_release() };
    {
        let mut write = rwlock.write().unwrap();
        *write += 5;
    }
    unsafe { read_rw.reobtain().unwrap() };
    assert_eq!(*read_rw, 10);
}

#[test]
fn slice_read() {
    let rwlock = MrwLock::new(vec![1, 2, 3]);
    let slice_read = rwlock.try_read_slice().unwrap();

    assert_eq!(*slice_read, [1, 2, 3])
}

#[test]
fn write_early_release() {
    let rwlock = MrwLock::new(Vec::from([1, 2, 3]));
    let mut write_rw = rwlock.write().unwrap();
    unsafe { write_rw.early_release() };
    {
        let mut write2 = rwlock.write().unwrap();
        write2.push(4);
    }
    unsafe { write_rw.reobtain().unwrap() };
    write_rw.push(5);
    assert_eq!(*write_rw, [1, 2, 3, 4, 5]);
}
