// okok so basically this hides borrow_mut from Rc<RefCell>

use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

#[derive(Clone)]
pub struct ReadonlyStateHandle<T> {
    state: Rc<RefCell<T>>,
}

impl<T> ReadonlyStateHandle<T> {
    pub fn over(rw: &ReadWriteStateHandle<T>) -> Self {
        Self {
            state: Rc::clone(&rw.state),
        }
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.state.borrow()
    }
}

#[derive(Default)]
pub struct ReadWriteStateHandle<T> {
    state: Rc<RefCell<T>>,
}

impl<T> ReadWriteStateHandle<T> {
    pub fn new(state: T) -> Self {
        Self {
            state: Rc::new(RefCell::new(state)),
        }
    }

    pub fn read(&self) -> Ref<'_, T> {
        self.state.borrow()
    }

    pub fn write(&self) -> RefMut<'_, T> {
        self.state.borrow_mut()
    }
}

impl<T> Clone for ReadWriteStateHandle<T> {
    fn clone(&self) -> Self {
        Self {
            state: Rc::clone(&self.state),
        }
    }
}

#[derive(Clone)]
pub struct ThreadedReadonlyStateHandle<T> {
    state: Arc<RwLock<T>>,
}

impl<T> ThreadedReadonlyStateHandle<T> {
    pub fn over(readwrite: &ThreadedReadWriteStateHandle<T>) -> Self {
        Self {
            state: Arc::clone(&readwrite.state),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.state.read().unwrap()
    }
}

#[derive(Default)]
pub struct ThreadedReadWriteStateHandle<T> {
    state: Arc<RwLock<T>>,
}

impl<T> ThreadedReadWriteStateHandle<T> {
    pub fn new(state: T) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.state.read().unwrap()
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.state.write().unwrap()
    }
}

impl<T> Clone for ThreadedReadWriteStateHandle<T> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}
