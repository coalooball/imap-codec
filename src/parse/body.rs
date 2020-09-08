use crate::{
    parse::{
        core::{nil, nstring, number, string},
        envelope::envelope,
    },
    types::{
        body::{
            BasicFields, Body, BodyStructure, MultiPartExtensionData, SinglePartExtensionData,
            SpecificFields,
        },
        core::{istr, nstr},
    },
};
use abnf_core::streaming::SP;
use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt, recognize},
    multi::{many0, many1, separated_nonempty_list},
    sequence::{delimited, preceded, tuple},
    IResult,
};

/// body = "(" (body-type-1part / body-type-mpart) ")"
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
pub(crate) fn body(remaining_recursions: usize) -> impl Fn(&[u8]) -> IResult<&[u8], BodyStructure> {
    move |input: &[u8]| body_limited(input, remaining_recursions)
}

fn body_limited<'a>(
    input: &'a [u8],
    remaining_recursions: usize,
) -> IResult<&'a [u8], BodyStructure> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(nom::error::make_error(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let body_type_1part = move |input: &'a [u8]| {
        body_type_1part_limited(input, remaining_recursions.saturating_sub(1))
    };
    let body_type_mpart = move |input: &'a [u8]| {
        body_type_mpart_limited(input, remaining_recursions.saturating_sub(1))
    };

    delimited(
        tag(b"("),
        alt((body_type_1part, body_type_mpart)),
        tag(b")"),
    )(input)
}

/// body-type-1part = (body-type-basic / body-type-msg / body-type-text) [SP body-ext-1part]
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed.
fn body_type_1part_limited<'a>(
    input: &'a [u8],
    remaining_recursions: usize,
) -> IResult<&'a [u8], BodyStructure> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(nom::error::make_error(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let body_type_msg =
        move |input: &'a [u8]| body_type_msg_limited(input, remaining_recursions.saturating_sub(1));

    let parser = tuple((
        alt((body_type_msg, body_type_text, body_type_basic)),
        opt(preceded(SP, body_ext_1part)),
    ));

    let (remaining, ((basic, specific), maybe_extension)) = parser(input)?;

    Ok((
        remaining,
        BodyStructure::Single {
            body: Body {
                basic,
                specific,
                extension: maybe_extension,
            },
        },
    ))
}

/// body-type-basic = media-basic SP body-fields
///
/// MESSAGE subtype MUST NOT be "RFC822"
fn body_type_basic(input: &[u8]) -> IResult<&[u8], (BasicFields, SpecificFields)> {
    let parser = tuple((media_basic, SP, body_fields));

    let (remaining, ((type_, subtype), _, basic)) = parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Basic {
                type_: type_.to_owned(),
                subtype: subtype.to_owned(),
            },
        ),
    ))
}

/// body-type-msg = media-message SP body-fields SP envelope SP body SP body-fld-lines
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
fn body_type_msg_limited<'a>(
    input: &'a [u8],
    remaining_recursions: usize,
) -> IResult<&'a [u8], (BasicFields, SpecificFields)> {
    if remaining_recursions == 0 {
        return Err(nom::Err::Failure(nom::error::make_error(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let body = move |input: &'a [u8]| body_limited(input, remaining_recursions.saturating_sub(1));

    let parser = tuple((
        media_message,
        SP,
        body_fields,
        SP,
        envelope,
        SP,
        body,
        SP,
        body_fld_lines,
    ));

    let (remaining, (_, _, basic, _, envelope, _, body_structure, _, number_of_lines)) =
        parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Message {
                envelope,
                body_structure: Box::new(body_structure),
                number_of_lines,
            },
        ),
    ))
}

/// body-type-text = media-text SP body-fields SP body-fld-lines
fn body_type_text(input: &[u8]) -> IResult<&[u8], (BasicFields, SpecificFields)> {
    let parser = tuple((media_text, SP, body_fields, SP, body_fld_lines));

    let (remaining, (subtype, _, basic, _, number_of_lines)) = parser(input)?;

    Ok((
        remaining,
        (
            basic,
            SpecificFields::Text {
                subtype: subtype.to_owned(),
                number_of_lines,
            },
        ),
    ))
}

/// body-fields = body-fld-param SP body-fld-id SP
///               body-fld-desc SP body-fld-enc SP
///               body-fld-octets
fn body_fields(input: &[u8]) -> IResult<&[u8], BasicFields> {
    let parser = tuple((
        body_fld_param,
        SP,
        body_fld_id,
        SP,
        body_fld_desc,
        SP,
        body_fld_enc,
        SP,
        body_fld_octets,
    ));

    let (remaining, (parameter_list, _, id, _, description, _, content_transfer_encoding, _, size)) =
        parser(input)?;

    Ok((
        remaining,
        BasicFields {
            parameter_list: parameter_list
                .iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            id: id.to_owned(),
            description: description.to_owned(),
            content_transfer_encoding: content_transfer_encoding.to_owned(),
            size,
        },
    ))
}

/// body-fld-param = "(" string SP string *(SP string SP string) ")" / nil
fn body_fld_param(input: &[u8]) -> IResult<&[u8], Vec<(istr, istr)>> {
    let parser = alt((
        delimited(
            tag(b"("),
            separated_nonempty_list(
                SP,
                map(tuple((string, SP, string)), |(key, _, value)| (key, value)),
            ),
            tag(b")"),
        ),
        map(nil, |_| vec![]),
    ));

    let (remaining, parsed_body_fld_param) = parser(input)?;

    Ok((remaining, parsed_body_fld_param))
}

#[inline]
/// body-fld-id = nstring
fn body_fld_id(input: &[u8]) -> IResult<&[u8], nstr> {
    nstring(input)
}

#[inline]
/// body-fld-desc = nstring
fn body_fld_desc(input: &[u8]) -> IResult<&[u8], nstr> {
    nstring(input)
}

#[inline]
/// body-fld-enc = (DQUOTE ("7BIT" / "8BIT" / "BINARY" / "BASE64"/ "QUOTED-PRINTABLE") DQUOTE) / string
///
/// Simplified...
///
/// body-fld-enc = string
///
/// TODO: why the special case?
fn body_fld_enc(input: &[u8]) -> IResult<&[u8], istr> {
    string(input)
}

#[inline]
/// body-fld-octets = number
fn body_fld_octets(input: &[u8]) -> IResult<&[u8], u32> {
    number(input)
}

#[inline]
/// body-fld-lines = number
fn body_fld_lines(input: &[u8]) -> IResult<&[u8], u32> {
    number(input)
}

/// body-ext-1part = body-fld-md5
///                  [SP body-fld-dsp
///                    [SP body-fld-lang
///                      [SP body-fld-loc *(SP body-extension)]
///                    ]
///                  ]
///
/// MUST NOT be returned on non-extensible "BODY" fetch
///
/// TODO: this is insane... define macro?
fn body_ext_1part(input: &[u8]) -> IResult<&[u8], SinglePartExtensionData> {
    let mut rem;
    let md5;
    let mut dsp = None;
    let mut lang = None;
    let mut loc = None;
    let mut ext = Vec::new();

    let (rem_, md5_) = body_fld_md5(input)?;
    rem = rem_;
    md5 = md5_;

    let (rem_, dsp_) = opt(preceded(SP, body_fld_dsp))(rem)?;
    if let Some(dsp_) = dsp_ {
        rem = rem_;
        dsp = Some(dsp_);

        let (rem_, lang_) = opt(preceded(SP, body_fld_lang))(rem)?;
        if let Some(lang_) = lang_ {
            rem = rem_;
            lang = Some(lang_);

            let (rem_, loc_) = opt(preceded(SP, body_fld_loc))(rem)?;
            if let Some(loc_) = loc_ {
                rem = rem_;
                loc = Some(loc_);

                let (rem_, ext_) = recognize(many0(preceded(SP, body_extension(8))))(rem)?;
                rem = rem_;
                ext = ext_.to_vec();
            }
        }
    }

    Ok((
        rem,
        SinglePartExtensionData {
            md5: md5.to_owned(),
            disposition: dsp.map(|inner| {
                inner.map(|(a, b)| {
                    (
                        a.to_owned(),
                        b.iter()
                            .map(|(key, value)| (key.to_owned(), value.to_owned()))
                            .collect(),
                    )
                })
            }),
            language: lang.map(|inner| inner.iter().map(|item| item.to_owned()).collect()),
            location: loc.map(|inner| inner.to_owned()),
            extension: ext,
        },
    ))
}

#[inline]
/// body-fld-md5 = nstring
fn body_fld_md5(input: &[u8]) -> IResult<&[u8], nstr> {
    nstring(input)
}

/// body-fld-dsp = "(" string SP body-fld-param ")" / nil
fn body_fld_dsp(input: &[u8]) -> IResult<&[u8], Option<(istr, Vec<(istr, istr)>)>> {
    alt((
        delimited(
            tag(b"("),
            map(
                tuple((string, SP, body_fld_param)),
                |(string, _, body_fld_param)| Some((string, body_fld_param)),
            ),
            tag(b")"),
        ),
        map(nil, |_| None),
    ))(input)
}

/// body-fld-lang = nstring / "(" string *(SP string) ")"
fn body_fld_lang(input: &[u8]) -> IResult<&[u8], Vec<istr>> {
    alt((
        map(nstring, |nstring| match nstring.0 {
            Some(item) => vec![item],
            None => vec![],
        }),
        delimited(tag(b"("), separated_nonempty_list(SP, string), tag(b")")),
    ))(input)
}

#[inline]
/// body-fld-loc = nstring
fn body_fld_loc(input: &[u8]) -> IResult<&[u8], nstr> {
    nstring(input)
}

/// Future expansion.
///
/// Client implementations MUST accept body-extension fields.
/// Server implementations MUST NOT generate body-extension fields except as defined by
/// future standard or standards-track revisions of this specification.
///
/// body-extension = nstring / number / "(" body-extension *(SP body-extension) ")"
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed. (8 should suffice).
///
/// TODO: This recognizes extension data and returns &[u8].
fn body_extension(remaining_recursions: usize) -> impl Fn(&[u8]) -> IResult<&[u8], &[u8]> {
    move |input: &[u8]| body_extension_limited(input, remaining_recursions)
}

fn body_extension_limited<'a>(
    input: &'a [u8],
    remaining_recursion: usize,
) -> IResult<&'a [u8], &[u8]> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(nom::error::make_error(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let body_extension =
        move |input: &'a [u8]| body_extension_limited(input, remaining_recursion.saturating_sub(1));

    alt((
        recognize(nstring),
        recognize(number),
        recognize(delimited(
            tag(b"("),
            separated_nonempty_list(SP, body_extension),
            tag(b")"),
        )),
    ))(input)
}

// ---

/// body-type-mpart = 1*body SP media-subtype [SP body-ext-mpart]
///
/// Note: This parser is recursively defined. Thus, in order to not overflow the stack,
/// it is needed to limit how may recursions are allowed.
fn body_type_mpart_limited(
    input: &[u8],
    remaining_recursion: usize,
) -> IResult<&[u8], BodyStructure> {
    if remaining_recursion == 0 {
        return Err(nom::Err::Failure(nom::error::make_error(
            input,
            nom::error::ErrorKind::TooLarge,
        )));
    }

    let parser = tuple((
        many1(body(remaining_recursion)),
        SP,
        media_subtype,
        opt(preceded(SP, body_ext_mpart)),
    ));

    let (remaining, (bodies, _, subtype, maybe_extension_data)) = parser(input)?;

    Ok((
        remaining,
        BodyStructure::Multi {
            bodies,
            subtype: subtype.to_owned(),
            extension_data: maybe_extension_data,
        },
    ))
}

/// body-ext-mpart = body-fld-param
///                  [SP body-fld-dsp
///                    [SP body-fld-lang
///                      [SP body-fld-loc *(SP body-extension)]
///                    ]
///                  ]
///
/// MUST NOT be returned on non-extensible "BODY" fetch
///
/// TODO: this is insane, too... define macro?
fn body_ext_mpart(input: &[u8]) -> IResult<&[u8], MultiPartExtensionData> {
    let mut rem;
    let param;
    let mut dsp = None;
    let mut lang = None;
    let mut loc = None;
    let mut ext = Vec::new();

    let (rem_, param_) = body_fld_param(input)?;
    rem = rem_;
    param = param_;

    let (rem_, dsp_) = opt(preceded(SP, body_fld_dsp))(rem)?;
    if let Some(dsp_) = dsp_ {
        rem = rem_;
        dsp = Some(dsp_);

        let (rem_, lang_) = opt(preceded(SP, body_fld_lang))(rem)?;
        if let Some(lang_) = lang_ {
            rem = rem_;
            lang = Some(lang_);

            let (rem_, loc_) = opt(preceded(SP, body_fld_loc))(rem)?;
            if let Some(loc_) = loc_ {
                rem = rem_;
                loc = Some(loc_);

                let (rem_, ext_) = recognize(many0(preceded(SP, body_extension(8))))(rem)?;
                rem = rem_;
                ext = ext_.to_vec();
            }
        }
    }

    Ok((
        rem,
        MultiPartExtensionData {
            parameter_list: param
                .iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
            disposition: dsp.map(|inner| {
                inner.map(|(a, b)| {
                    (
                        a.to_owned(),
                        b.iter()
                            .map(|(key, value)| (key.to_owned(), value.to_owned()))
                            .collect(),
                    )
                })
            }),
            language: lang.map(|inner| inner.iter().map(|item| item.to_owned()).collect()),
            location: loc.map(|inner| inner.to_owned()),
            extension: ext,
        },
    ))
}

// ---

/// media-basic = ((DQUOTE ("APPLICATION" / "AUDIO" / "IMAGE" / "MESSAGE" / "VIDEO") DQUOTE) / string) SP media-subtype
///
/// Simplified...
///
/// media-basic = string SP media-subtype
///
/// TODO: Why the special case?
///
/// Defined in [MIME-IMT]
fn media_basic(input: &[u8]) -> IResult<&[u8], (istr, istr)> {
    let parser = tuple((string, SP, media_subtype));

    let (remaining, (type_, _, subtype)) = parser(input)?;

    Ok((remaining, (type_, subtype)))
}

#[inline]
/// media-subtype = string
///
/// Defined in [MIME-IMT]
fn media_subtype(input: &[u8]) -> IResult<&[u8], istr> {
    string(input)
}

#[inline]
/// media-message = DQUOTE "MESSAGE" DQUOTE SP DQUOTE "RFC822" DQUOTE
///
/// Simplified:
///
/// media-message = "\"MESSAGE\" \"RFC822\""
///
/// Defined in [MIME-IMT]
///
/// "message" "rfc822" basic specific-for-message-rfc822 extension
fn media_message(input: &[u8]) -> IResult<&[u8], &[u8]> {
    tag_no_case(b"\"MESSAGE\" \"RFC822\"")(input)
}

/// media-text = DQUOTE "TEXT" DQUOTE SP media-subtype
///
/// Defined in [MIME-IMT]
///
/// "text" "?????" basic specific-for-text extension
fn media_text(input: &[u8]) -> IResult<&[u8], istr> {
    let parser = preceded(tag_no_case(b"\"TEXT\" "), media_subtype);

    let (remaining, media_subtype) = parser(input)?;

    Ok((remaining, media_subtype))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_media_basic() {
        media_basic(b"\"application\" \"xxx\"").unwrap();
        media_basic(b"\"unknown\" \"test\"").unwrap();
        media_basic(b"\"x\" \"xxx\"").unwrap();
    }

    #[test]
    fn test_media_message() {
        media_message(b"\"message\" \"rfc822\"").unwrap();
    }

    #[test]
    fn test_media_text() {
        media_text(b"\"text\" \"html\"").unwrap();
    }

    #[test]
    fn test_body_ext_1part() {
        for test in [
            b"nil|xxx".as_ref(),
            b"\"md5\"|xxx".as_ref(),
            b"\"md5\" nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" nil)|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) \"swedish\"|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\")|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") nil|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\"|xxx".as_ref(),
            b"\"md5\" (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" (1 \"2\" (nil 4))|xxx".as_ref(),
        ]
        .iter()
        {
            let (rem, out) = body_ext_1part(test).unwrap();
            println!("{:?}", out);
            assert_eq!(rem, b"|xxx");
        }
    }

    #[test]
    fn test_body_rec() {
        let _ = body(8)(str::repeat("(", 1_000_000).as_bytes());
    }

    #[test]
    fn test_body_ext_mpart() {
        for test in [
            b"nil|xxx".as_ref(),
            b"(\"key\" \"value\")|xxx".as_ref(),
            b"(\"key\" \"value\") nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" nil)|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) \"swedish\"|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\")|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") nil|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\"|xxx".as_ref(),
            b"(\"key\" \"value\") (\"dsp\" (\"key\" \"value\")) (\"german\" \"russian\") \"loc\" (1 \"2\" (nil 4))|xxx".as_ref(),
        ]
            .iter()
        {
            let (rem, out) = body_ext_mpart(test).unwrap();
            println!("{:?}", out);
            assert_eq!(rem, b"|xxx");
        }
    }
}
