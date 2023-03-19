use core::fmt;

use bstr::{BStr, ByteSlice};

use crate::yaml::data::Data;
use crate::yaml::raw::Raw;
use crate::yaml::{Any, Mapping, Sequence};

use super::data::ValueId;

/// Separator to use when separating the value from its key or sequence marker.
///
/// ```yaml
/// -   hello
/// - world
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Separator<'a> {
    /// Automatically figure out which separator to use based on the last
    /// element in the collection.
    ///
    /// If this does not exist, a default separator of `" "` will be used.
    Auto,
    /// A custom separator.
    ///
    /// # Legal separators
    ///
    /// The only legal separator in YAML is spaces, but this can technically
    /// contain anything and will be literally embedded in the generated YAML.
    /// It is up to the caller to ensure nothing but spaces is used or suffer
    /// the consequences.
    Custom(&'a str),
}

/// The kind of a null value.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum NullKind {
    /// A keyword `null` value.
    Keyword,
    /// A tilde `~` null value.
    Tilde,
    /// A empty null value.
    Empty,
}

impl NullKind {
    pub(crate) fn display(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NullKind::Keyword => {
                write!(f, "null")?;
            }
            NullKind::Tilde => {
                write!(f, "~")?;
            }
            NullKind::Empty => {
                // empty values count as null.
            }
        }

        Ok(())
    }
}

/// A value inside of the document.
///
/// # Examples
///
/// ```
/// use nondestructive::yaml;
///
/// let doc = yaml::from_bytes("string")?;
/// assert_eq!(doc.root().as_str(), Some("string"));
///
/// let doc = yaml::from_bytes("\"a double-quoted string\"")?;
/// assert_eq!(doc.root().as_str(), Some("a double-quoted string"));
///
/// let doc = yaml::from_bytes("'a single-quoted string'")?;
/// assert_eq!(doc.root().as_str(), Some("a single-quoted string"));
///
/// let doc = yaml::from_bytes("'It''s a bargain!'")?;
/// assert_eq!(doc.root().as_str(), Some("It's a bargain!"));
///
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
pub struct Value<'a> {
    pub(crate) data: &'a Data,
    pub(crate) id: ValueId,
}

macro_rules! as_number {
    ($name:ident, $ty:ty, $doc:literal, $lit:literal) => {
        #[doc = concat!("Try and get the value as a ", $doc, ".")]
        ///
        /// # Examples
        ///
        /// ```
        /// use nondestructive::yaml;
        ///
        #[doc = concat!("let doc = yaml::from_bytes(\"", stringify!($lit), "\")?;")]
        #[doc = concat!("let value = doc.root().", stringify!($name), "();")]
        #[doc = concat!("assert_eq!(value, Some(", stringify!($lit), "));")]
        /// # Ok::<_, Box<dyn std::error::Error>>(())
        /// ```
        #[must_use]
        pub fn $name(&self) -> Option<$ty> {
            match self.data.raw(self.id) {
                Raw::Number(raw) => {
                    let string = self.data.str(raw.string);
                    lexical_core::parse(string).ok()
                }
                _ => None,
            }
        }
    };
}

impl<'a> Value<'a> {
    pub(crate) fn new(data: &'a Data, id: ValueId) -> Self {
        Self { data, id }
    }

    /// Coerce into [`Any`] to help discriminate the value type.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
    /// Hello World
    /// "#)?;
    ///
    /// assert!(matches!(doc.root().into_any(), yaml::Any::Scalar(..)));
    ///
    /// let doc = yaml::from_bytes(r#"
    /// number1: 10
    /// number2: 20
    /// "#)?;
    ///
    /// assert!(matches!(doc.root().into_any(), yaml::Any::Mapping(..)));
    ///
    /// let doc = yaml::from_bytes(r#"
    /// - 10
    /// - 20
    /// "#)?;
    ///
    /// assert!(matches!(doc.root().into_any(), yaml::Any::Sequence(..)));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn into_any(self) -> Any<'a> {
        match self.data.raw(self.id) {
            Raw::Mapping(..) => Any::Mapping(Mapping::new(self.data, self.id)),
            Raw::Sequence(..) => Any::Sequence(Sequence::new(self.data, self.id)),
            _ => Any::Scalar(self),
        }
    }

    /// Get the opaque [`ValueId`] associated with this value.
    ///
    /// This can be used through [`Document::value`] to look up the same value
    /// again.
    ///
    /// [`Document::value`]: crate::yaml::Document::value
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
    /// first: 32
    /// second: [1, 2, 3]
    /// "#)?;
    ///
    /// let root = doc.root().as_mapping().ok_or("missing mapping")?;
    /// let second = root.get("second").ok_or("missing second")?;
    /// let id = second.id();
    ///
    /// // Reference the same value again using the id.
    /// assert_eq!(doc.value(id).to_string(), "[1, 2, 3]");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    #[inline]
    pub fn id(&self) -> ValueId {
        self.id
    }

    /// Get the value as a [`BStr`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    /// use bstr::BStr;
    ///
    /// let doc = yaml::from_bytes("string")?;
    /// assert_eq!(doc.root().as_str(), Some("string"));
    ///
    /// let doc = yaml::from_bytes(r#"
    /// - It's the same string!
    /// - "It's the same string!"
    /// - 'It''s the same string!'
    /// "#)?;
    ///
    /// let array = doc.root().as_sequence().ok_or("expected sequence")?;
    ///
    /// for item in array {
    ///     assert_eq!(item.as_bstr(), Some(BStr::new("It's the same string!")));
    /// }
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_bstr(self) -> Option<&'a BStr> {
        match self.data.raw(self.id) {
            Raw::String(raw) => Some(self.data.str(raw.string)),
            _ => None,
        }
    }

    /// Get the value as a [`str`]. This might fail if the underlying string is
    /// not valid UTF-8.
    ///
    /// See [`Value::as_bstr`] for an alternative.
    ///
    /// # Escape sequences and unicode
    ///
    /// YAML supports a variety of escape sequences which will be handled by
    /// this parser.
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes("フェリスと言います！")?;
    /// assert_eq!(doc.root().as_str(), Some("フェリスと言います！"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes("\"hello \\x20 world\"")?;
    /// assert_eq!(doc.root().as_str(), Some("hello \x20 world"));
    /// assert_eq!(doc.to_string(), "\"hello \\x20 world\"");
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes("string")?;
    /// assert_eq!(doc.root().as_str(), Some("string"));
    ///
    /// let doc = yaml::from_bytes(r#"
    /// - It's the same string!
    /// - "It's the same string!"
    /// - 'It''s the same string!'
    /// "#)?;
    ///
    /// let array = doc.root().as_sequence().ok_or("expected sequence")?;
    ///
    /// for item in array {
    ///     assert_eq!(item.as_str(), Some("It's the same string!"));
    /// }
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_str(self) -> Option<&'a str> {
        match self.data.raw(self.id) {
            Raw::String(raw) => self.data.str(raw.string).to_str().ok(),
            _ => None,
        }
    }

    /// Get the value as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes("true")?;
    /// assert_eq!(doc.root().as_bool(), Some(true));
    ///
    /// let doc = yaml::from_bytes("string")?;
    /// assert_eq!(doc.root().as_bool(), None);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self.data.raw(self.id) {
            Raw::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// Get the value as a [`Mapping`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(r#"
    /// number1: 10
    /// number2: 20
    /// mapping:
    ///   inner: 400
    /// string3: "I am a quoted string!"
    /// "#)?;
    ///
    /// let root = doc.root().as_mapping().ok_or("missing root mapping")?;
    ///
    /// assert_eq!(root.get("number1").and_then(|v| v.as_u32()), Some(10));
    /// assert_eq!(root.get("number2").and_then(|v| v.as_u32()), Some(20));
    ///
    /// let mapping = root.get("mapping").and_then(|v| v.as_mapping()).ok_or("missing inner mapping")?;
    /// assert_eq!(mapping.get("inner").and_then(|v| v.as_u32()), Some(400));
    ///
    /// assert_eq!(root.get("string3").and_then(|v| v.as_str()), Some("I am a quoted string!"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_mapping(&self) -> Option<Mapping<'a>> {
        match self.data.raw(self.id) {
            Raw::Mapping(..) => Some(Mapping::new(self.data, self.id)),
            _ => None,
        }
    }

    /// Get the value as a [`Sequence`].
    ///
    /// # Examples
    ///
    /// ```
    /// use nondestructive::yaml;
    ///
    /// let doc = yaml::from_bytes(
    ///     r#"
    ///     - one
    ///     - two
    ///     - three
    ///     "#,
    /// )?;
    ///
    /// let root = doc.root().as_sequence().ok_or("missing root sequence")?;
    ///
    /// assert_eq!(root.get(0).and_then(|v| v.as_str()), Some("one"));
    /// assert_eq!(root.get(1).and_then(|v| v.as_str()), Some("two"));
    /// assert_eq!(root.get(2).and_then(|v| v.as_str()), Some("three"));
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_sequence(&self) -> Option<Sequence<'a>> {
        match self.data.raw(self.id) {
            Raw::Sequence(..) => Some(Sequence::new(self.data, self.id)),
            _ => None,
        }
    }

    as_number!(as_f32, f32, "32-bit float", 10.42);
    as_number!(as_f64, f64, "64-bit float", 10.42);
    as_number!(as_u8, u8, "8-bit unsigned integer", 42);
    as_number!(as_i8, i8, "8-bit signed integer", -42);
    as_number!(as_u16, u16, "16-bit unsigned integer", 42);
    as_number!(as_i16, i16, "16-bit signed integer", -42);
    as_number!(as_u32, u32, "16-bit unsigned integer", 42);
    as_number!(as_i32, i32, "32-bit signed integer", -42);
    as_number!(as_u64, u64, "16-bit unsigned integer", 42);
    as_number!(as_i64, i64, "64-bit signed integer", -42);
    as_number!(as_u128, u128, "16-bit unsigned integer", 42);
    as_number!(as_i128, i128, "128-bit signed integer", -42);
}

impl fmt::Display for Value<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.raw(self.id).display(self.data, f)
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Display<'a, 'b>(&'a Value<'b>);

        impl fmt::Debug for Display<'_, '_> {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.data.raw(self.0.id).display(self.0.data, f)
            }
        }

        f.debug_tuple("Value").field(&Display(self)).finish()
    }
}
