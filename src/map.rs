//! Generic TOML map types.

use std::collections::{btree_map, BTreeMap};

use crate::Value;

/// A generic map type.
pub type Map<K, V> = BTreeMap<K, V>;

/// A TOML table type.
pub type Table = Map<String, Value>;

/// An iterator over a [`Table`]'s values.
pub type Iter<'a> = btree_map::Iter<'a, String, Value>;

/// A mutable iterator over a [`Table`]'s values.
pub type IterMut<'a> = btree_map::IterMut<'a, String, Value>;

/// An owning iterator over a [`Table`]'s values.
pub type IntoIter = btree_map::IntoIter<String, Value>;

/// An iterator over a [`Table`]'s keys.
pub type Keys<'a> = btree_map::Keys<'a, String, Value>;

/// An iterator over a [`Table`]'s values.
pub type Values<'a> = btree_map::Values<'a, String, Value>;

/// A mutable iterator over a [`Table`]'s values.
pub type ValuesMut<'a> = btree_map::ValuesMut<'a, String, Value>;

/// An owning iterator over a [`Table`]'s keys.
pub type IntoKeys = btree_map::IntoKeys<String, Value>;

/// An owning iterator over a [`Table`]'s values.
pub type IntoValues = btree_map::IntoValues<String, Value>;

/// A single entry in [`Table`].
pub type Entry<'a> = btree_map::Entry<'a, String, Value>;

/// A vacant entry in [`Table`].
pub type VacantEntry<'a> = btree_map::VacantEntry<'a, String, Value>;

/// An occupied entry in [`Table`].
pub type OccupiedEntry<'a> = btree_map::OccupiedEntry<'a, String, Value>;
