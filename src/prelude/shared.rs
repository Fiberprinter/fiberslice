use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{render::model::Model, render::Renderable};

#[derive(Default, Debug)]
pub struct SharedMut<T: std::fmt::Debug> {
    inner: Arc<RwLock<T>>,
}

impl<T: std::fmt::Debug> Clone for SharedMut<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: std::fmt::Debug> SharedMut<T> {
    pub fn from_inner(inner: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    pub fn read(&self) -> RwLockReadGuard<T> {
        self.inner.read()
    }

    pub fn read_with_fn<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let inner = self.inner.read();
        f(&*inner)
    }

    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.inner.write()
    }

    pub fn write_with_fn<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut inner = self.inner.write();
        f(&mut *inner)
    }
}

pub type Shared<T> = Arc<T>;

#[derive(Debug)]
pub struct LockModel<T: std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable> {
    inner: RwLock<Model<T>>,
}

impl<T: std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable> LockModel<T> {
    pub fn new(inner: Model<T>) -> Self {
        Self {
            inner: RwLock::new(inner),
        }
    }

    pub fn read(&self) -> RwLockReadGuard<Model<T>> {
        self.inner.read()
    }

    pub fn read_with_fn<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Model<T>) -> R,
    {
        let inner = self.inner.read();
        f(&*inner)
    }

    pub fn write(&self) -> RwLockWriteGuard<Model<T>> {
        self.inner.write()
    }

    pub fn write_with_fn<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Model<T>) -> R,
    {
        let mut inner = self.inner.write();
        f(&mut *inner)
    }

    pub fn get_mut(&mut self) -> &mut Model<T> {
        self.inner.get_mut()
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let inner = self.inner.read();

        unsafe {
            self.inner.data_ptr().as_ref().unwrap().render(render_pass);
        }
        drop(inner);
    }

    pub fn render_without_color<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        let inner = self.inner.read();

        unsafe {
            self.inner
                .data_ptr()
                .as_ref()
                .unwrap()
                .render_without_color(render_pass);
        }
        drop(inner);
    }
}
