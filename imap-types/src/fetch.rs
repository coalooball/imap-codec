use std::num::NonZeroU32;

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;
#[cfg(feature = "bounded-static")]
use bounded_static::ToStatic;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    body::BodyStructure, core::NString, datetime::DateTime, envelope::Envelope, flag::FlagFetch,
    section::Section,
};

/// Shorthands for commonly-used message data items.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[cfg_attr(feature = "bounded-static", derive(ToStatic))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Macro {
    /// Shorthand for `(FLAGS INTERNALDATE RFC822.SIZE)`.
    Fast,
    /// Shorthand for `(FLAGS INTERNALDATE RFC822.SIZE ENVELOPE)`.
    All,
    /// Shorthand for `(FLAGS INTERNALDATE RFC822.SIZE ENVELOPE BODY)`.
    Full,
}

impl Macro {
    pub fn expand(&self) -> Vec<FetchAttribute> {
        use FetchAttribute::*;

        match self {
            Self::All => vec![Flags, InternalDate, Rfc822Size, Envelope],
            Self::Fast => vec![Flags, InternalDate, Rfc822Size],
            Self::Full => vec![Flags, InternalDate, Rfc822Size, Envelope, Body],
        }
    }
}

/// Either a macro or a list of message data items.
///
/// A macro must be used by itself, and not in conjunction with other macros or data items.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[cfg_attr(feature = "bounded-static", derive(ToStatic))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroOrFetchAttributes<'a> {
    Macro(Macro),
    FetchAttributes(Vec<FetchAttribute<'a>>),
}

impl<'a> From<Macro> for MacroOrFetchAttributes<'a> {
    fn from(m: Macro) -> Self {
        MacroOrFetchAttributes::Macro(m)
    }
}

impl<'a> From<Vec<FetchAttribute<'a>>> for MacroOrFetchAttributes<'a> {
    fn from(attributes: Vec<FetchAttribute<'a>>) -> Self {
        MacroOrFetchAttributes::FetchAttributes(attributes)
    }
}

/// Message data item name used to request a message data item.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[cfg_attr(feature = "bounded-static", derive(ToStatic))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FetchAttribute<'a> {
    /// Non-extensible form of `BODYSTRUCTURE`.
    ///
    /// ```imap
    /// BODY
    /// ```
    Body,

    /// The text of a particular body section.
    ///
    /// ```imap
    /// BODY[<section>]<<partial>>
    /// ```
    BodyExt {
        /// The section specification is a set of zero or more part specifiers delimited by periods.
        ///
        /// An empty section specification refers to the entire message, including the header.
        ///
        /// See [`crate::section::Section`] and [`crate::section::PartSpecifier`].
        ///
        /// Every message has at least one part number.  Non-[MIME-IMB]
        /// messages, and non-multipart [MIME-IMB] messages with no
        /// encapsulated message, only have a part 1.
        ///
        /// Multipart messages are assigned consecutive part numbers, as
        /// they occur in the message.  If a particular part is of type
        /// message or multipart, its parts MUST be indicated by a period
        /// followed by the part number within that nested multipart part.
        ///
        /// A part of type MESSAGE/RFC822 also has nested part numbers,
        /// referring to parts of the MESSAGE part's body.
        section: Option<Section<'a>>,
        /// It is possible to fetch a substring of the designated text.
        /// This is done by appending an open angle bracket ("<"), the
        /// octet position of the first desired octet, a period, the
        /// maximum number of octets desired, and a close angle bracket
        /// (">") to the part specifier.  If the starting octet is beyond
        /// the end of the text, an empty string is returned.
        ///
        /// Any partial fetch that attempts to read beyond the end of the
        /// text is truncated as appropriate.  A partial fetch that starts
        /// at octet 0 is returned as a partial fetch, even if this
        /// truncation happened.
        ///
        ///    Note: This means that BODY[]<0.2048> of a 1500-octet message
        ///    will return BODY[]<0> with a literal of size 1500, not
        ///    BODY[].
        ///
        ///    Note: A substring fetch of a HEADER.FIELDS or
        ///    HEADER.FIELDS.NOT part specifier is calculated after
        ///    subsetting the header.
        partial: Option<(u32, NonZeroU32)>,
        /// Defines, wheather BODY or BODY.PEEK should be used.
        ///
        /// `BODY[...]` implicitly sets the `\Seen` flag where `BODY.PEEK[...]` does not.
        peek: bool,
    },

    /// The [MIME-IMB] body structure of a message.
    ///
    /// This is computed by the server by parsing the [MIME-IMB] header fields in the [RFC-2822]
    /// header and [MIME-IMB] headers.
    ///
    /// ```imap
    /// BODYSTRUCTURE
    /// ```
    BodyStructure,

    /// The envelope structure of a message.
    ///
    /// This is computed by the server by parsing the [RFC-2822] header into the component parts,
    /// defaulting various fields as necessary.
    ///
    /// ```imap
    /// ENVELOPE
    /// ```
    Envelope,

    /// The flags that are set for a message.
    ///
    /// ```imap
    /// FLAGS
    /// ```
    Flags,

    /// The internal date of a message.
    ///
    /// ```imap
    /// INTERNALDATE
    /// ```
    InternalDate,

    /// Functionally equivalent to `BODY[]`.
    ///
    /// Differs in the syntax of the resulting untagged FETCH data (`RFC822` is returned).
    ///
    /// ```imap
    /// RFC822
    /// ```
    ///
    /// Note: `BODY[]` is constructed as ...
    ///
    /// ```rust
    /// # use imap_types::fetch::MessageDataItemName;
    /// MessageDataItemName::BodyExt {
    ///     section: None,
    ///     partial: None,
    ///     peek: false,
    /// };
    /// ```
    Rfc822,

    /// Functionally equivalent to `BODY.PEEK[HEADER]`.
    ///
    /// Differs in the syntax of the resulting untagged FETCH data (`RFC822.HEADER` is returned).
    ///
    /// ```imap
    /// RFC822.HEADER
    /// ```
    Rfc822Header,

    /// The [RFC-2822] size of a message.
    ///
    /// ```imap
    /// RFC822.SIZE
    /// ```
    Rfc822Size,

    /// Functionally equivalent to `BODY[TEXT]`.
    ///
    /// Differs in the syntax of the resulting untagged FETCH data (`RFC822.TEXT` is returned).
    /// ```imap
    /// RFC822.TEXT
    /// ```
    Rfc822Text,

    /// The unique identifier for a message.
    ///
    /// ```imap
    /// UID
    /// ```
    Uid,
}

/// Message data item.
#[cfg_attr(feature = "arbitrary", derive(Arbitrary))]
#[cfg_attr(feature = "bounded-static", derive(ToStatic))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FetchAttributeValue<'a> {
    /// A form of `BODYSTRUCTURE` without extension data.
    ///
    /// ```imap
    /// BODY
    /// ```
    Body(BodyStructure<'a>),

    /// The body contents of the specified section.
    ///
    /// 8-bit textual data is permitted if a \[CHARSET\] identifier is
    /// part of the body parameter parenthesized list for this section.
    /// Note that headers (part specifiers HEADER or MIME, or the
    /// header portion of a MESSAGE/RFC822 part), MUST be 7-bit; 8-bit
    /// characters are not permitted in headers.  Note also that the
    /// [RFC-2822] delimiting blank line between the header and the
    /// body is not affected by header line subsetting; the blank line
    /// is always included as part of header data, except in the case
    /// of a message which has no body and no blank line.
    ///
    /// Non-textual data such as binary data MUST be transfer encoded
    /// into a textual form, such as BASE64, prior to being sent to the
    /// client.  To derive the original binary data, the client MUST
    /// decode the transfer encoded string.
    ///
    /// ```imap
    /// BODY[<section>]<<origin octet>>
    /// ```
    BodyExt {
        /// The specified section.
        section: Option<Section<'a>>,
        /// If the origin octet is specified, this string is a substring of
        /// the entire body contents, starting at that origin octet.  This
        /// means that `BODY[]<0>` MAY be truncated, but `BODY[]` is NEVER
        /// truncated.
        ///
        ///    Note: The origin octet facility MUST NOT be used by a server
        ///    in a FETCH response unless the client specifically requested
        ///    it by means of a FETCH of a `BODY[<section>]<<partial>>` data
        ///    item.
        origin: Option<u32>,
        /// The string SHOULD be interpreted by the client according to the
        /// content transfer encoding, body type, and subtype.
        data: NString<'a>,
    },

    /// The [MIME-IMB] body structure of a message.
    ///
    /// This is computed by the server by parsing the [MIME-IMB] header fields, defaulting various
    /// fields as necessary.
    ///
    /// ```imap
    /// BODYSTRUCTURE
    /// ```
    BodyStructure(BodyStructure<'a>),

    /// The envelope structure of a message.
    ///
    /// This is computed by the server by parsing the [RFC-2822] header into the component parts,
    /// defaulting various fields as necessary.
    ///
    /// ```imap
    /// ENVELOPE
    /// ```
    Envelope(Envelope<'a>),

    /// A list of flags that are set for a message.
    ///
    /// ```imap
    /// FLAGS
    /// ```
    Flags(Vec<FlagFetch<'a>>),

    /// A string representing the internal date of a message.
    ///
    /// ```imap
    /// INTERNALDATE
    /// ```
    InternalDate(DateTime),

    /// Equivalent to `BODY[]`.
    ///
    /// ```imap
    /// RFC822
    /// ```
    Rfc822(NString<'a>),

    /// Equivalent to `BODY[HEADER]`.
    ///
    /// Note that this did not result in `\Seen` being set, because `RFC822.HEADER` response data
    /// occurs as a result of a `FETCH` of `RFC822.HEADER`. `BODY[HEADER]` response data occurs as a
    /// result of a `FETCH` of `BODY[HEADER]` (which sets `\Seen`) or `BODY.PEEK[HEADER]` (which
    /// does not set `\Seen`).
    ///
    /// ```imap
    /// RFC822.HEADER
    /// ```
    Rfc822Header(NString<'a>),

    /// A number expressing the [RFC-2822] size of a message.
    ///
    /// ```imap
    /// RFC822.SIZE
    /// ```
    Rfc822Size(u32),

    /// Equivalent to `BODY[TEXT]`.
    ///
    /// ```imap
    /// RFC822.TEXT
    /// ```
    Rfc822Text(NString<'a>),

    /// A number expressing the unique identifier of a message.
    ///
    /// ```imap
    /// UID
    /// ```
    Uid(NonZeroU32),
}
