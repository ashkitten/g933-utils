//! Some helpful utility types and stuff

use std::ops::Deref;

/// A struct that derefs to its contents
pub struct DerefInner<T>(pub T);

impl<T> Deref for DerefInner<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}
