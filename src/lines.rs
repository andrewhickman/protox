use std::convert::TryFrom;

use logos::Span;

pub(crate) struct LineResolver {
    lines: Vec<usize>,
}

impl LineResolver {
    pub fn new(source_code: &str) -> Self {
        let lines = source_code
            .match_indices('\n')
            .map(|(index, _)| index + 1)
            .collect();
        LineResolver { lines }
    }

    pub fn resolve(&self, offset: usize) -> (usize, usize) {
        match self.lines.binary_search(&offset) {
            Ok(index) => (index + 1, 0),
            Err(0) => (0, offset),
            Err(index) => (index, offset - self.lines[index - 1]),
        }
    }

    pub fn resolve_span(&self, span: Span) -> Vec<i32> {
        let (start_line, start_col) = self.resolve(span.start);
        let (end_line, end_col) = self.resolve(span.end);

        if start_line == end_line {
            vec![to_i32(start_line), to_i32(start_col), to_i32(end_col)]
        } else {
            vec![
                to_i32(start_col),
                to_i32(start_col),
                to_i32(end_line),
                to_i32(end_col),
            ]
        }
    }
}

fn to_i32(i: usize) -> i32 {
    i32::try_from(i).unwrap()
}

#[test]
fn resolve_line_number() {
    let resolver = LineResolver::new("hello\nworld\nfoo");

    dbg!(&resolver.lines);

    assert_eq!(resolver.resolve(0), (0, 0));
    assert_eq!(resolver.resolve(4), (0, 4));
    assert_eq!(resolver.resolve(5), (0, 5));
    assert_eq!(resolver.resolve(6), (1, 0));
    assert_eq!(resolver.resolve(7), (1, 1));
    assert_eq!(resolver.resolve(10), (1, 4));
    assert_eq!(resolver.resolve(11), (1, 5));
    assert_eq!(resolver.resolve(12), (2, 0));
    assert_eq!(resolver.resolve(13), (2, 1));
    assert_eq!(resolver.resolve(14), (2, 2));
    assert_eq!(resolver.resolve(15), (2, 3));
}
