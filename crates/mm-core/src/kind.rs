//! The kinds of content a Content ID can name, as types for [`Id`](crate::Id).

/// A kind of content: the `<kind>` segment of a Content ID.
pub trait Kind: 'static {
    const NAME: &'static str;
}

/// A class (`class/…`).
#[derive(Debug, Clone, Copy)]
pub enum Class {}

/// An item, weapons included (`item/…`).
#[derive(Debug, Clone, Copy)]
pub enum Item {}

impl Kind for Class {
    const NAME: &'static str = "class";
}

impl Kind for Item {
    const NAME: &'static str = "item";
}
