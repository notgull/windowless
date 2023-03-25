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
use core::cmp;

use slotmap::{new_key_type, SlotMap};
use smallvec::SmallVec;
use tinyvec::{ArrayVec, TinyVec};

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
        self.windows
            .iter()
            .map(|(key, window)| (WindowKey(key), window.rect))
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
    pub fn insert(&mut self, rect: Rectangle) -> Result<WindowKey, InsertError> {
        use alloc::vec;

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
                return Ok(key);
            }
        };

        // We should be able to fit this window in the root window.
        if rect.intersects(self.windows[root.0].rect) {
            return Err(InsertError::OutsideRoot);
        }
        let root = match self.root {
            Some(root) => root,
            None => {
                self.root = Some(key);
                return Ok(key);
            }
        };

        // Get the intersections.
        let (parents, _) = self.intersections(&[root], rect);

        // Set children.
        for parent in &parents {
            self.windows[parent.0].children.push(key);
        }

        // The current rectangles and windows we're testing.
        let mut current_rects = tinyvec::tiny_vec![[Rectangle; 4] => rect];
        let mut current_windows: SmallVec<[WindowKey; 3]> = smallvec::smallvec![root];

        while !current_rects.is_empty() {
            // Take the rectangles.
            let mut taken_rects = core::mem::take(&mut current_rects);
            let mut taken_windows = core::mem::take(&mut current_windows);

            // Drain the rectangles.
            while !taken_rects.is_empty() {
                let rect = taken_rects.swap_remove(0);

                // Find the intersection of the rectangle with the windows.
                for window in &taken_windows {
                    let window_rect = self.windows[window.0].rect;
                    let (intersection, remainder) = match rect.intersection(window_rect) {
                        Some(x) => x,
                        None => continue,
                    };

                    // This window's children should be carried over to the next iteration.
                }
            }
        }

        Ok(key)
    }

    /// Get the windows that this rectangle intersects, at this root.
    fn intersections(
        &self,
        roots: &[WindowKey],
        rect: Rectangle,
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
                let (mut child_intersections, leftovers) =
                    self.intersections(&self.windows[matched_root.0].children, intersection);

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

/// Error with inserting a window.
#[derive(Debug)]
pub enum InsertError {
    /// This window falls outside of the bounds of the root window.
    OutsideRoot,
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

    fn area(&self) -> i32 {
        self.width() * self.height()
    }

    /// Tell if two windows intersect.
    fn intersects(&self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }

    /// Intersect this rectangle with another.
    ///
    /// Returns the intersection and the remainder of the `Self` rectangle.
    fn intersection(mut self, other: Self) -> Option<(Self, ArrayVec<[Self; 4]>)> {
        // See if the rectangles intersect.
        if self.intersects(other) {
            return None;
        }

        let mut remainder = ArrayVec::new();

        // See if the bottom edge of this rectangle intersects the top edge of the other.
        if self.bottom > other.top {
            let new_top = cmp::max(self.top, other.top);

            // Push the remainder of the top edge if there is any.
            if new_top != self.top {
                remainder.push(Rectangle {
                    bottom: new_top,
                    ..self
                });
            }

            self.top = new_top;
        }

        // See if the top edge of this rectangle intersects the bottom edge of the other.
        if self.top < other.bottom {
            let new_bottom = cmp::min(self.bottom, other.bottom);

            // Push the remainder of the bottom edge if there is any.
            if new_bottom != self.bottom {
                remainder.push(Rectangle {
                    top: new_bottom,
                    ..self
                });
            }

            self.bottom = new_bottom;
        }

        // See if the left edge of this rectangle intersects the right edge of the other.
        if self.left < other.right {
            let new_right = cmp::min(self.right, other.right);

            // Push the remainder of the right edge if there is any.
            if new_right != self.right {
                remainder.push(Rectangle {
                    left: new_right,
                    ..self
                });
            }

            self.right = new_right;
        }

        // See if the right edge of this rectangle intersects the left edge of the other.
        if self.right > other.left {
            let new_left = cmp::max(self.left, other.left);

            // Push the remainder of the left edge if there is any.
            if new_left != self.left {
                remainder.push(Rectangle {
                    right: new_left,
                    ..self
                });
            }

            self.left = new_left;
        }

        Some((self, remainder))
    }
}

#[cfg(test)]
mod tests {
    use super::{Rectangle, WindowTable};

    #[test]
    fn no_intersect() {
        let a = Rectangle::new(0, 0, 10, 10);
        let b = Rectangle::new(15, 15, 25, 25);

        assert!(a.intersection(b).is_none());
    }

    #[test]
    fn intersect_inside() {
        let a = Rectangle::new(5, 5, 10, 10);
        let b = Rectangle::new(0, 0, 15, 15);

        let (intersection, remainder) = a.intersection(b).unwrap();
        assert_eq!(intersection, a);
        assert!(remainder.is_empty());
    }

    #[test]
    fn intersect_corner() {
        let a = Rectangle::new(0, 0, 10, 10);
        let b = Rectangle::new(5, 5, 15, 15);

        let (intersection, remainder) = a.intersection(b).unwrap();

        assert_eq!(intersection, Rectangle::new(5, 5, 10, 10));
        assert_eq!(remainder.len(), 2);
    }

    #[test]
    fn insert() {
        let mut window_table = WindowTable::new();

        // Inserting a single rectangle should install it as the root.
        let rect1 = window_table.insert(Rectangle::new(0, 0, 100, 100)).unwrap();
        assert_eq!(Some(rect1), window_table.root);

        {
            let slot = &window_table.windows[rect1.0];
            assert!(slot.children.is_empty());
            assert!(slot.parents.is_empty());
        }
    }
}
