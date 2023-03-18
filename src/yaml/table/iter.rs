use core::slice;

use bstr::BStr;

use crate::yaml::data::Data;
use crate::yaml::raw::RawTableItem;
use crate::yaml::Value;

/// An immutable iterator over a [`Table`][crate::yaml::table::Table].
///
/// See [`Table::iter`][crate::yaml::table::Table::iter].
pub struct Iter<'a> {
    data: &'a Data,
    iter: slice::Iter<'a, RawTableItem>,
}

impl<'a> Iter<'a> {
    #[inline]
    pub(crate) fn new(data: &'a Data, slice: &'a [RawTableItem]) -> Self {
        Self {
            data,
            iter: slice.iter(),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a BStr, Value<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        let key = self.data.str(&item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        let key = self.data.str(&item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for Iter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.iter.next_back()?;
        let key = self.data.str(&item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let item = self.iter.nth(n)?;
        let key = self.data.str(&item.key.string);
        let value = Value::new(self.data, item.value);
        Some((key, value))
    }
}

impl ExactSizeIterator for Iter<'_> {}
