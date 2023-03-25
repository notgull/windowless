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

extern crate alloc;

use alloc::vec;

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
        let root = match self.root {
            Some(root) => root,
            None => {
                self.root = Some(key);
                return key;
            }
        };

        // Get the intersections.
        let (parents, _) = self.intersections(&[root], rect);

        // Set children.
        for parent in &parents {
            self.windows[parent.0].children.push(key);
        }

        // Set parents.
        self.windows[key.0].parents = parents;

        key
    }

    /// Get the windows that this rectangle intersects, at this root.
    fn intersections(
        &self,
        roots: &[WindowKey],
        rect: Rectangle
    ) -> (SmallVec<[WindowKey; 3]>, bool) {
        let mut windows = SmallVec::new();
        let mut rectangles = tinyvec::tiny_vec![[Rectangle; 4] => rect];
        let mut leftovers = true;

        while !rectangles.is_empty() {
            let rect = rectangles.swap_remove(0);

            let intersect = roots.iter().find_map(|root| {
                let window = self.windows[root.0].rect;
                rect.intersection(window).map(|(i, r)| (i, r, root))
            });

            if let Some((intersection, remainder, matched_root)) = intersect {
                // Re-run this code on the new root's children.
                let (mut child_intersections, leftovers) = self.intersections(
                    &self.windows[matched_root.0].children,
                    intersection
                );

                // Append the results to our list.
                windows.append(&mut child_intersections);

                // If there are leftover rectangles, this root intersects.
                if leftovers {
                    windows.push(*matched_root);
                }

                // Try again for remainder.
                rectangles.append(&mut tinyvec::TinyVec::Inline(remainder));
            } else {
                // There are leftover rectangles here.
                leftovers = true;
            }
        }

        // Remove duplicates.
        windows.sort_unstable();
        windows.dedup();

        (windows, leftovers)
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
    ) -> Option<(Self, ArrayVec<[Self; 4]>)> {
        todo!() 
    }
}

#[cfg(test)]
mod tests {
    use super::{Rectangle, WindowTable};

    #[test]
    fn intersect() {
        let a = Rectangle::new(0, 0, 10, 10);
        let b = Rectangle::new(5, 5, 15, 15);

        let (a, b) = a.intersection(b).unwrap();

        assert_eq!(a, Rectangle::new(5, 5, 10, 10));
        assert_eq!(b.as_slice(), &[Rectangle::new(10, 5, 15, 15)]);
    }

    #[test]
    fn insert() {
        let mut window_table = WindowTable::new();

        // Inserting a single rectangle should install it as the root.
        let rect1 = window_table.insert(Rectangle::new(0, 0, 100, 100));
        assert_eq!(Some(rect1), window_table.root);
        
        {
            let slot = &window_table.windows[rect1.0];
            assert!(slot.children.is_empty());
            assert!(slot.parents.is_empty());
        }
    }
}
