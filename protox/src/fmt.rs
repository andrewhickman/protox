use std::fmt::{self, Write};

pub(crate) struct HexEscaped<'a>(pub &'a [u8]);

impl<'a> fmt::Display for HexEscaped<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &ch in self.0 {
            match ch {
                b'\t' => f.write_str("\\t")?,
                b'\r' => f.write_str("\\r")?,
                b'\n' => f.write_str("\\n")?,
                b'\\' => f.write_str("\\\\")?,
                b'\'' => f.write_str("\\'")?,
                b'"' => f.write_str("\\\"")?,
                b'\x20'..=b'\x7e' => f.write_char(ch as char)?,
                _ => {
                    write!(f, "\\{:03o}", ch)?;
                }
            }
        }

        Ok(())
    }
}
