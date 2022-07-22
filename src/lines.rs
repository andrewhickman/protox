use logos::Span;

use crate::index_to_i32;

#[derive(Debug, Clone)]
pub(crate) struct LineResolver {
    lines: Vec<i32>,
}

impl LineResolver {
    pub fn new(source_code: &str) -> Self {
        let lines = source_code
            .match_indices('\n')
            .map(|(index, _)| index_to_i32(index + 1))
            .collect();
        LineResolver { lines }
    }

    pub fn resolve(&self, offset: usize) -> (i32, i32) {
        match self.lines.binary_search(&index_to_i32(offset)) {
            Ok(index) => (index_to_i32(index + 1), 0),
            Err(0) => (0, index_to_i32(offset)),
            Err(index) => (
                index_to_i32(index),
                index_to_i32(offset) - self.lines[index - 1],
            ),
        }
    }

    pub fn resolve_span(&self, span: Span) -> Vec<i32> {
        let (start_line, start_col) = self.resolve(span.start);
        let (end_line, end_col) = self.resolve(span.end);

        if start_line == end_line {
            vec![start_line, start_col, end_col]
        } else {
            vec![start_line, start_col, end_line, end_col]
        }
    }

    pub fn resolve_proto(&self, line: i32, col: i32) -> Option<usize> {
        if line == 0 {
            Some(col as usize)
        } else {
            Some((self.lines.get((line - 1) as usize)? + col) as usize)
        }
    }

    pub fn resolve_proto_span(&self, span: &[i32]) -> Option<Span> {
        let (start_line, start_col, end_line, end_col) = match *span {
            [start_line, start_col, end_col] => (start_line, start_col, start_line, end_col),
            [start_line, start_col, end_line, end_col] => {
                (start_line, start_col, end_line, end_col)
            }
            _ => return None,
        };

        let start = self.resolve_proto(start_line, start_col)?;
        let end = self.resolve_proto(end_line, end_col)?;

        Some(start..end)
    }
}

#[test]
fn resolve_line_number() {
    let resolver = LineResolver::new("hello\nworld\nfoo");

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

#[test]
fn roundtrip_span() {
    let resolver = LineResolver::new("hello\nworld\nfoo");

    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(0..5))
            .unwrap(),
        0..5
    );
    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(4..6))
            .unwrap(),
        4..6
    );
    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(7..11))
            .unwrap(),
        7..11
    );
    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(10..12))
            .unwrap(),
        10..12
    );
    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(13..15))
            .unwrap(),
        13..15
    );
    assert_eq!(
        resolver
            .resolve_proto_span(&resolver.resolve_span(14..14))
            .unwrap(),
        14..14
    );
}
