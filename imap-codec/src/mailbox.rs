use abnf_core::streaming::{dquote, sp};
use imap_types::{
    core::QuotedChar,
    flag::FlagNameAttribute,
    mailbox::{ListCharString, ListMailbox, Mailbox},
    response::Data,
    utils::indicators::is_list_char,
};
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case, take_while1},
    combinator::{map, opt, value},
    multi::many0,
    sequence::{delimited, preceded, tuple},
};

use crate::{
    core::{astring, nil, number, nz_number, quoted_char, string},
    decode::IMAPResult,
    extensions::quota::{quota_response, quotaroot_response},
    flag::{flag_list, mbx_list_flags},
    status::status_att_list,
};

/// `list-mailbox = 1*list-char / string`
pub(crate) fn list_mailbox(input: &[u8]) -> IMAPResult<&[u8], ListMailbox> {
    alt((
        map(take_while1(is_list_char), |bytes: &[u8]| {
            // # Safety
            //
            // `unwrap` is safe here, because `is_list_char` enforces that the bytes ...
            //   * contain ASCII-only characters, i.e., `from_utf8` will return `Ok`.
            //   * are valid according to `ListCharString::verify()`, i.e., `unvalidated` is safe.
            ListMailbox::Token(ListCharString::unvalidated(
                std::str::from_utf8(bytes).unwrap(),
            ))
        }),
        map(string, ListMailbox::String),
    ))(input)
}

/// `mailbox = "INBOX" / astring`
///
/// INBOX is case-insensitive. All case variants of INBOX (e.g., "iNbOx")
/// MUST be interpreted as INBOX not as an astring.
///
/// An astring which consists of the case-insensitive sequence
/// "I" "N" "B" "O" "X" is considered to be INBOX and not an astring.
///
/// Refer to section 5.1 for further semantic details of mailbox names.
pub(crate) fn mailbox(input: &[u8]) -> IMAPResult<&[u8], Mailbox> {
    map(astring, Mailbox::from)(input)
}

/// `mailbox-data = "FLAGS" SP flag-list /
///                 "LIST" SP mailbox-list /
///                 "LSUB" SP mailbox-list /
///                 "SEARCH" *(SP nz-number) /
///                 "STATUS" SP mailbox SP "(" [status-att-list] ")" /
///                 number SP "EXISTS" /
///                 number SP "RECENT"`
pub(crate) fn mailbox_data(input: &[u8]) -> IMAPResult<&[u8], Data> {
    alt((
        map(
            tuple((tag_no_case(b"FLAGS"), sp, flag_list)),
            |(_, _, flags)| Data::Flags(flags),
        ),
        map(
            tuple((tag_no_case(b"LIST"), sp, mailbox_list)),
            |(_, _, (items, delimiter, mailbox))| Data::List {
                items: items.unwrap_or_default(),
                mailbox,
                delimiter,
            },
        ),
        map(
            tuple((tag_no_case(b"LSUB"), sp, mailbox_list)),
            |(_, _, (items, delimiter, mailbox))| Data::Lsub {
                items: items.unwrap_or_default(),
                mailbox,
                delimiter,
            },
        ),
        map(
            tuple((tag_no_case(b"SEARCH"), many0(preceded(sp, nz_number)))),
            |(_, nums)| Data::Search(nums),
        ),
        map(
            tuple((
                tag_no_case(b"STATUS"),
                sp,
                mailbox,
                sp,
                delimited(tag(b"("), opt(status_att_list), tag(b")")),
            )),
            |(_, _, mailbox, _, items)| Data::Status {
                mailbox,
                items: items.unwrap_or_default().into(),
            },
        ),
        map(
            tuple((number, sp, tag_no_case(b"EXISTS"))),
            |(num, _, _)| Data::Exists(num),
        ),
        map(
            tuple((number, sp, tag_no_case(b"RECENT"))),
            |(num, _, _)| Data::Recent(num),
        ),
        quotaroot_response,
        quota_response,
    ))(input)
}

/// `mailbox-list = "(" [mbx-list-flags] ")" SP
///                 (DQUOTE QUOTED-CHAR DQUOTE / nil) SP
///                 mailbox`
#[allow(clippy::type_complexity)]
pub(crate) fn mailbox_list(
    input: &[u8],
) -> IMAPResult<&[u8], (Option<Vec<FlagNameAttribute>>, Option<QuotedChar>, Mailbox)> {
    let mut parser = tuple((
        delimited(tag(b"("), opt(mbx_list_flags), tag(b")")),
        sp,
        alt((
            map(delimited(dquote, quoted_char, dquote), Option::Some),
            value(None, nil),
        )),
        sp,
        mailbox,
    ));

    let (remaining, (mbx_list_flags, _, maybe_delimiter, _, mailbox)) = parser(input)?;

    Ok((remaining, (mbx_list_flags, maybe_delimiter, mailbox)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mailbox() {
        assert!(mailbox(b"\"iNbOx\"").is_ok());
        assert!(mailbox(b"{3}\r\naaa\r\n").is_ok());
        assert!(mailbox(b"inbox ").is_ok());
        assert!(mailbox(b"inbox.sent ").is_ok());
        assert!(mailbox(b"aaa").is_err());
    }
}
