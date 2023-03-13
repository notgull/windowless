// SPDX-License-Identifier: LGPL-3.0-or-later OR MPL-2.0
// This file is a part of `windowless`.
//
// `windowless` is free software: you can redistribute it and/or modify it under the terms of
// either:
//
// * GNU Lesser General Public License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
// * Mozilla Public License as published by the Mozilla Foundation, version 2.
//
// `windowless` is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU Lesser General Public License or the Mozilla Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public License and the Mozilla
// Public License along with `windowless`. If not, see <https://www.gnu.org/licenses/> or
// <https://www.mozilla.org/en-US/MPL/2.0/>.

//! A table for creating virtual windows.

#![forbid(unsafe_code)]
#![no_std]

use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use tinyvec::ArrayVec;

/// The key type for windows.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowKey(Key);

new_key_type! {
    struct Key;
}

/// The table of windows
#[derive(Debug, Default)]
pub struct WindowTable {
    /// The windows.
    windows: SlotMap<Key, Window>,

    /// The root window.
    root: Option<WindowKey>,
}

/// The window.
#[derive(Debug)]
struct Window {
    /// The rectangle (LTRB) of the window.
    rect: Rectangle,

    /// The parents of the window.
    parents: SmallVec<[WindowKey; 3]>,

    /// The children of the window.
    children: SmallVec<[WindowKey; 3]>,
}

impl WindowTable {
    /// Creates a new window table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate over the windows.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (WindowKey, Rectangle)> + '_ {
        self.windows.iter().map(|(key, window)| {
            (
                WindowKey(key),
                window.rect
            )
        })
    }

    /// Empties the window table.
    pub fn clear(&mut self) {
        self.windows.clear();
        self.root = None;
    }

    /// Returns the root window.
    pub fn root(&self) -> Option<WindowKey> {
        self.root
    }

    /// Insert a new window.
    pub fn insert(&mut self, rect: Rectangle) -> WindowKey {
        let key = {
            let inner = self.windows.insert(Window {
                rect,
                parents: SmallVec::new(),
                children: SmallVec::new(),
            });
            
            WindowKey(inner)
        };

        // If there is no root window, set this window as the root.
        if self.root.is_none() {
            self.root = Some(key);
            return key;
        }

        let slot = self.windows.get_mut(key.0).unwrap();
        todo!();

        key
    }
}

/// The current cursor state.
#[derive(Debug)]
pub struct CursorState {
    /// Last known cursor position.
    position: (i32, i32),

    /// Windows currently under the cursor.
    windows: SmallVec<[WindowKey; 3]>,
}

/// A rectangle.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Rectangle {
    /// The left coordinate.
    pub left: i32,

    /// The top coordinate.
    pub top: i32,

    /// The right coordinate.
    pub right: i32,

    /// The bottom coordinate.
    pub bottom: i32,
}

impl Rectangle {
    /// Creates a new rectangle.
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Returns the width of the rectangle.
    pub fn width(&self) -> i32 {
        (self.right - self.left).abs()
    }

    /// Returns the height of the rectangle.
    pub fn height(&self) -> i32 {
        (self.bottom - self.top).abs()
    }

    /// Intersect this rectangle with another.
    fn intersection(
        mut self,
        other: Self
    ) -> Option<(Self, ArrayVec<[Self; 3]>)> {
        todo!() 
    }
}

#[cfg(test)]
mod tests {
    use super::Rectangle;

    #[test]
    fn intersect() {
        let a = Rectangle::new(0, 0, 10, 10);
        let b = Rectangle::new(5, 5, 15, 15);

        let (a, b) = a.intersection(b).unwrap();

        assert_eq!(a, Rectangle::new(5, 5, 10, 10));
        assert_eq!(b.as_slice(), &[Rectangle::new(10, 5, 15, 15)]);
    }
}
