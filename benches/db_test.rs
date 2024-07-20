use std::{
  ops::{Deref, DerefMut},
  path::Path,
};

pub struct DBTest<TDB> {
  inner: Option<TDB>,
  on_drop: Option<Box<dyn FnOnce() + Send + Sync>>,
}

impl<TDB> Drop for DBTest<TDB> {
  fn drop(&mut self) {
    #[allow(unsafe_code)]
    unsafe {
      drop(self.inner.take().unwrap_unchecked());
    }

    if let Some(x) = self.on_drop.take() {
      x();
    }
  }
}

impl<TDB> DBTest<TDB> {
  pub fn new(inner: TDB, on_drop: Option<Box<dyn FnOnce() + Send + Sync>>) -> Self {
    Self {
      inner: Some(inner),
      on_drop,
    }
  }
}

impl<TDB> Deref for DBTest<TDB> {
  type Target = TDB;

  fn deref(&self) -> &Self::Target {
    #[allow(unsafe_code)]
    unsafe {
      self.inner.as_ref().unwrap_unchecked()
    }
  }
}

impl<TDB> DerefMut for DBTest<TDB> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    #[allow(unsafe_code)]
    unsafe {
      self.inner.as_mut().unwrap_unchecked()
    }
  }
}

/// Handle drop errors.
fn handle_fs_errors_on_drop(res: std::io::Result<()>) {
  if let Err(err) = res {
    eprintln!("{err}");
  }
}

/// Handle drop event.
pub fn handle_on_drop_remove_dir(x: impl AsRef<Path>) {
  let path = x.as_ref();

  if path.is_dir() {
    handle_fs_errors_on_drop(std::fs::remove_dir_all(path));
  } else if path.is_file() {
    handle_fs_errors_on_drop(std::fs::remove_file(path));
  }
}
