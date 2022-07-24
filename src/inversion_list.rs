use std::{cmp::Ordering, ops::Range};

#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct InversionList {
    ranges: Vec<Range<i32>>,
}

impl InversionList {
    pub fn new(ranges: impl IntoIterator<Item = Range<i32>>) -> Self {
        let mut ranges: Vec<_> = ranges
            .into_iter()
            .filter(|range| !range.is_empty())
            .collect();
        ranges.sort_by_key(|range| range.start);
        ranges.dedup_by(|hi, lo| {
            debug_assert!(lo.start <= hi.start);
            debug_assert!(lo.start < lo.end);
            debug_assert!(hi.start < hi.end);
            if hi.start <= lo.end {
                lo.end = hi.end;
                true
            } else {
                false
            }
        });

        InversionList { ranges }
    }

    pub fn contains(&self, item: i32) -> bool {
        self.ranges
            .binary_search_by(|range| {
                if item < range.start {
                    Ordering::Greater
                } else if item < range.end {
                    Ordering::Equal
                } else {
                    Ordering::Less
                }
            })
            .is_ok()
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    pub fn as_slice(&self) -> &[Range<i32>] {
        self.ranges.as_slice()
    }
}

#[test]
#[allow(clippy::reversed_empty_ranges)]
fn test() {
    let list = InversionList::new([7..-2, 1..4, 9..12, 3..6, 5..5, 12..13]);

    assert!(!list.contains(0));
    assert!(list.contains(1));
    assert!(list.contains(2));
    assert!(list.contains(3));
    assert!(list.contains(4));
    assert!(list.contains(5));
    assert!(!list.contains(6));
    assert!(!list.contains(7));
    assert!(!list.contains(8));
    assert!(list.contains(9));
    assert!(list.contains(10));
    assert!(list.contains(11));
    assert!(list.contains(12));
    assert!(!list.contains(13));
    assert!(!list.contains(14));
}
