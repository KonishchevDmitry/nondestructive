use std::fmt::{self, Write};

use bstr::ByteSlice;

use crate::strings::{StringId, Strings};
use crate::yaml::{NullKind, StringKind};

/// A raw value.
#[derive(Debug, Clone)]
pub(crate) enum Raw {
    /// A null value.
    Null(NullKind),
    /// A single number.
    Number(RawNumber),
    /// A string.
    String(RawString),
    /// A table.
    Table(RawTable),
    /// A list.
    List(RawList),
}

impl Raw {
    pub(crate) fn display(&self, strings: &Strings, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use std::fmt::Display;

        match self {
            Raw::Null(raw) => {
                match raw {
                    NullKind::Keyword => {
                        "null".fmt(f)?;
                    }
                    NullKind::Tilde => {
                        "~".fmt(f)?;
                    }
                    NullKind::Empty => {
                        // empty values count as null.
                    }
                }
            }
            Raw::Number(raw) => {
                strings.get(&raw.string).fmt(f)?;
            }
            Raw::String(raw) => {
                let string = strings.get(&raw.string);

                match raw.kind {
                    StringKind::Bare => {
                        string.fmt(f)?;
                    }
                    StringKind::DoubleQuoted => {
                        escape_double_quoted(string, f)?;
                    }
                    StringKind::SingleQuoted => {
                        escape_single_quoted(string, f)?;
                    }
                }
            }
            Raw::Table(raw) => {
                for item in &raw.items {
                    if let Some(prefix) = &item.prefix {
                        strings.get(prefix).fmt(f)?;
                    }

                    strings.get(&item.key.string).fmt(f)?;
                    ':'.fmt(f)?;
                    strings.get(&item.separator).fmt(f)?;
                    item.value.display(strings, f)?;
                }
            }
            Raw::List(raw) => {
                for item in &raw.items {
                    if let Some(prefix) = &item.prefix {
                        strings.get(prefix).fmt(f)?;
                    }

                    '-'.fmt(f)?;
                    strings.get(&item.separator).fmt(f)?;
                    item.value.display(strings, f)?;
                }
            }
        }

        Ok(())
    }
}

/// Single-quoted escape sequences:
/// <https://yaml.org/spec/1.2.2/#escaped-characters>.
fn escape_single_quoted(string: &bstr::BStr, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_char('\'')?;

    for c in string.chars() {
        match c {
            '\'' => {
                f.write_str("''")?;
            }
            c => {
                f.write_char(c)?;
            }
        }
    }

    f.write_char('\'')?;
    Ok(())
}

/// Double-quoted escape sequences:
/// <https://yaml.org/spec/1.2.2/#escaped-characters>.
fn escape_double_quoted(string: &bstr::BStr, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    f.write_char('"')?;

    for c in string.chars() {
        match c {
            '\u{0000}' => {
                f.write_str("\\0")?;
            }
            '\u{0007}' => {
                f.write_str("\\a")?;
            }
            '\u{0008}' => {
                f.write_str("\\b")?;
            }
            '\u{0009}' => {
                f.write_str("\\t")?;
            }
            '\n' => {
                f.write_str("\\n")?;
            }
            '\u{000b}' => {
                f.write_str("\\v")?;
            }
            '\u{000c}' => {
                f.write_str("\\f")?;
            }
            '\r' => {
                f.write_str("\\r")?;
            }
            '\u{001b}' => {
                f.write_str("\\e")?;
            }
            '\"' => {
                f.write_str("\\\"")?;
            }
            c if c.is_ascii_control() => {
                write!(f, "\\x{:02x}", c as u8)?;
            }
            c => {
                f.write_char(c)?;
            }
        }
    }

    f.write_char('"')?;
    Ok(())
}

/// A YAML number.
#[derive(Debug, Clone)]
pub(crate) struct RawNumber {
    pub(crate) string: StringId,
}

impl RawNumber {
    /// A simple number.
    pub(crate) fn new(string: StringId) -> Self {
        Self { string }
    }
}

/// A YAML string.
#[derive(Debug, Clone)]
pub(crate) struct RawString {
    pub(crate) kind: StringKind,
    pub(crate) string: StringId,
}

impl RawString {
    /// A simple number.
    pub(crate) fn new(kind: StringKind, string: StringId) -> Self {
        Self { kind, string }
    }
}

/// An element in a YAML list.
#[derive(Debug, Clone)]
pub(crate) struct RawListItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) separator: StringId,
    pub(crate) value: Box<Raw>,
}

/// A YAML list.
#[derive(Debug, Clone)]
pub(crate) struct RawList {
    /// Indentation used by list.
    #[allow(unused)]
    pub(crate) indentation: StringId,
    /// Items in the list.
    pub(crate) items: Vec<RawListItem>,
}

impl RawList {
    /// Push a value on the list.
    pub(crate) fn push(&mut self, separator: StringId, value: Raw) {
        let prefix = if !self.items.is_empty() {
            Some(self.indentation)
        } else {
            None
        };

        self.items.push(RawListItem {
            prefix,
            separator,
            value: Box::new(value),
        });
    }
}

/// An element in a YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTableItem {
    pub(crate) prefix: Option<StringId>,
    pub(crate) key: RawString,
    pub(crate) separator: StringId,
    pub(crate) value: Box<Raw>,
}

/// A YAML table.
#[derive(Debug, Clone)]
pub(crate) struct RawTable {
    pub(crate) indentation: StringId,
    pub(crate) items: Vec<RawTableItem>,
}

impl RawTable {
    /// Insert a value into the table.
    pub(crate) fn insert(&mut self, key: RawString, separator: StringId, value: Raw) {
        if let Some(existing) = self.items.iter_mut().find(|c| c.key.string == key.string) {
            existing.separator = separator;
            existing.value = Box::new(value);
            return;
        }

        let prefix = if !self.items.is_empty() {
            Some(self.indentation)
        } else {
            None
        };

        self.items.push(RawTableItem {
            prefix,
            key,
            separator,
            value: Box::new(value),
        });
    }
}

/// Insert a boolean value.
pub(crate) fn insert_bool(strings: &mut Strings, value: bool) -> Raw {
    const TRUE: &[u8] = b"true";
    const FALSE: &[u8] = b"false";

    let string = strings.insert(if value { TRUE } else { FALSE });
    Raw::String(RawString::new(StringKind::Bare, string))
}
