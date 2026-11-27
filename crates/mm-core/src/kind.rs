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

/// A species (`species/…`).
#[derive(Debug, Clone, Copy)]
pub enum Species {}

/// A background (`background/…`).
#[derive(Debug, Clone, Copy)]
pub enum Background {}

/// A feat, Origin feats and fighting styles included (`feat/…`).
#[derive(Debug, Clone, Copy)]
pub enum Feat {}

/// A weapon mastery property (`mastery/…`).
#[derive(Debug, Clone, Copy)]
pub enum Mastery {}

impl Kind for Species {
    const NAME: &'static str = "species";
}

impl Kind for Background {
    const NAME: &'static str = "background";
}

impl Kind for Feat {
    const NAME: &'static str = "feat";
}

impl Kind for Mastery {
    const NAME: &'static str = "mastery";
}
