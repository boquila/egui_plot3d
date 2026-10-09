use std::{borrow::Cow, sync::Arc};

/// A column of plot attributes or indices, borrowed, owned, or cheaply shared.
///
/// Use an `Arc<[T]>` to reuse large style/index buffers across frames. This does
/// not cache coordinate bounds; use [`crate::PointBuffer`] for coordinates.
#[derive(Clone, Debug)]
pub enum PlotValues<'a, T> {
    /// Borrow an existing slice without allocating.
    Borrowed(&'a [T]),
    /// Own a vector.
    Owned(Vec<T>),
    /// Share immutable data across items and frames.
    Shared(Arc<[T]>),
}
impl<T> Default for PlotValues<'_, T> {
    fn default() -> Self {
        Self::Borrowed(&[])
    }
}
impl<T> PlotValues<'_, T> {
    /// Borrow the data regardless of its storage type.
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Borrowed(v) => v,
            Self::Owned(v) => v,
            Self::Shared(v) => v,
        }
    }
    /// Number of entries.
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }
    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}
impl<'a, T> From<&'a [T]> for PlotValues<'a, T> {
    fn from(v: &'a [T]) -> Self {
        Self::Borrowed(v)
    }
}
impl<'a, T, const N: usize> From<&'a [T; N]> for PlotValues<'a, T> {
    fn from(v: &'a [T; N]) -> Self {
        Self::Borrowed(v)
    }
}
impl<'a, T> From<&'a Vec<T>> for PlotValues<'a, T> {
    fn from(v: &'a Vec<T>) -> Self {
        Self::Borrowed(v)
    }
}
impl<T> From<Vec<T>> for PlotValues<'_, T> {
    fn from(v: Vec<T>) -> Self {
        Self::Owned(v)
    }
}
impl<T, const N: usize> From<[T; N]> for PlotValues<'_, T> {
    fn from(v: [T; N]) -> Self {
        Self::Owned(Vec::from(v))
    }
}
impl<T> From<Arc<[T]>> for PlotValues<'_, T> {
    fn from(v: Arc<[T]>) -> Self {
        Self::Shared(v)
    }
}
impl<T> From<&Arc<[T]>> for PlotValues<'_, T> {
    fn from(v: &Arc<[T]>) -> Self {
        Self::Shared(v.clone())
    }
}
impl<'a, T: Clone> From<Cow<'a, [T]>> for PlotValues<'a, T> {
    fn from(v: Cow<'a, [T]>) -> Self {
        match v {
            Cow::Borrowed(v) => Self::Borrowed(v),
            Cow::Owned(v) => Self::Owned(v),
        }
    }
}
