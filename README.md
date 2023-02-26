[![Build & Test](https://github.com/duesee/imap-codec/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/duesee/imap-codec/actions/workflows/build_and_test.yml)
[![Audit](https://github.com/duesee/imap-codec/actions/workflows/audit.yml/badge.svg)](https://github.com/duesee/imap-codec/actions/workflows/audit.yml)
[![Coverage](https://coveralls.io/repos/github/duesee/imap-codec/badge.svg?branch=main)](https://coveralls.io/github/duesee/imap-codec?branch=main)
[![Documentation](https://docs.rs/imap-codec/badge.svg)](https://docs.rs/imap-codec)

# imap-codec

This library provides parsing, serialization, and support for [IMAP4rev1] implementations.
It is based on [imap-types] and aims to become a rock-solid building block for IMAP client and server implementations in Rust.
The complete [formal syntax] of IMAP4rev1 and several IMAP extensions are implemented.
Please see the [documentation] for more information.

## Features

* Rust's type system is used to enforce correctness and make the library misuse-resistant.
It should not be possible to construct messages that violate the IMAP specification.
* Fuzzing (via [cargo fuzz]) and property-based tests are used to uncover parsing and serialization bugs.
For example, the library is fuzz-tested never to produce a message it can not parse itself.
* Every parser works in streaming mode, i.e., all parsers will return `Incomplete` when there is insufficient data to make a final decision. 
No command or response will ever be truncated.

## Usage

```rust
use imap_codec::{
    codec::{Decode, Encode},
    command::Command,
};

fn main() {
    let input = b"ABCD UID FETCH 1,2:* (BODY.PEEK[1.2.3.4.MIME]<42.1337>)\r\n";

    let (remainder, parsed) = Command::decode(input).unwrap();
    println!("Parsed:\n{:#?}\n", parsed);

    let mut buffer = Vec::new();
    parsed.encode(&mut buffer).unwrap(); // This could be send over the network.
    
    // Note: Not every IMAP message is valid UTF-8.
    //       We ignore that here to print the message.
    println!("Serialized:\n{}", String::from_utf8(buffer).unwrap());
}
```

## Examples

### Simple parsing

Try one of the `parse_*` examples, e.g., ...

```sh
$ cargo run --example=parse_command
```

... to parse some IMAP messages.

### Tokio demo

You can also start the [demo server] with ...

```sh
$ cd assets/demos/tokio_server
$ cargo run
```

... and connect to it with ...

```sh
$ netcat -C 127.0.0.1 14300
```

There is also a [demo client] available.

**Note:** All demos are a work-in-progress. Feel free to propose API changes to imap-codec (or imap-types) to simplify them.

### Parsed and serialized IMAP4rev1 connection

The following output was generated by reading the trace from [RFC 3501 section 8](https://tools.ietf.org/html/rfc3501#section-8), printing the input (first line), `Debug`-printing the parsed object (second line), and printing the serialized output (third line).

```rust
// * OK IMAP4rev1 Service Ready
Status(Ok { tag: None, code: None, text: Text("IMAP4rev1 Service Ready") })
// * OK IMAP4rev1 Service Ready

// a001 login mrc secret
Command { tag: Tag("a001"), body: Login { username: Atom(Atom("mrc")), password: Atom(Atom("secret")) } }
// a001 LOGIN mrc secret

// a001 OK LOGIN completed
Status(Ok { tag: Some(Tag("a001")), code: None, text: Text("LOGIN completed") })
// a001 OK LOGIN completed

// a002 select inbox
Command { tag: Tag("a002"), body: Select { mailbox: Inbox } }
// a002 SELECT INBOX

// * 18 EXISTS
Data(Exists(18))
// * 18 EXISTS

// * FLAGS (\Answered \Flagged \Deleted \Seen \Draft)
Data(Flags([Answered, Flagged, Deleted, Seen, Draft]))
// * FLAGS (\Answered \Flagged \Deleted \Seen \Draft)

// * 2 RECENT
Data(Recent(2))
// * 2 RECENT

// * OK [UNSEEN 17] Message 17 is the first unseen message
Status(Ok { tag: None, code: Some(Unseen(17)), text: Text("Message 17 is the first unseen message") })
// * OK [UNSEEN 17] Message 17 is the first unseen message

// * OK [UIDVALIDITY 3857529045] UIDs valid
Status(Ok { tag: None, code: Some(UidValidity(3857529045)), text: Text("UIDs valid") })
// * OK [UIDVALIDITY 3857529045] UIDs valid

// a002 OK [READ-WRITE] SELECT completed
Status(Ok { tag: Some(Tag("a002")), code: Some(ReadWrite), text: Text("SELECT completed") })
// a002 OK [READ-WRITE] SELECT completed

// a003 fetch 12 full
Command { tag: Tag("a003"), body: Fetch { sequence_set: SequenceSet([Single(Value(12))]), attributes: Macro(Full), uid: false } }
// a003 FETCH 12 FULL

// * 12 FETCH (FLAGS (\Seen) INTERNALDATE "17-Jul-1996 02:44:25 -0700" RFC822.SIZE 4286 ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US")("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") BODY ("TEXT" "PLAIN" ("CHARSET" "US-ASCII") NIL NIL "7BIT" 3028 92))
Data(Fetch { seq_or_uid: 12, attributes: [Flags([Seen]), InternalDate(1996-07-17T02:44:25-07:00), Rfc822Size(4286), Envelope(Envelope { date: NString(Some(Quoted(Quoted("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)")))), subject: NString(Some(Quoted(Quoted("IMAP4rev1 WG mtg summary and minutes")))), from: [Address { name: NString(Some(Quoted(Quoted("Terry Gray")))), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("gray")))), host: NString(Some(Quoted(Quoted("cac.washington.edu")))) }], sender: [Address { name: NString(Some(Quoted(Quoted("Terry Gray")))), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("gray")))), host: NString(Some(Quoted(Quoted("cac.washington.edu")))) }], reply_to: [Address { name: NString(Some(Quoted(Quoted("Terry Gray")))), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("gray")))), host: NString(Some(Quoted(Quoted("cac.washington.edu")))) }], to: [Address { name: NString(None), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("imap")))), host: NString(Some(Quoted(Quoted("cac.washington.edu")))) }], cc: [Address { name: NString(None), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("minutes")))), host: NString(Some(Quoted(Quoted("CNRI.Reston.VA.US")))) }, Address { name: NString(Some(Quoted(Quoted("John Klensin")))), adl: NString(None), mailbox: NString(Some(Quoted(Quoted("KLENSIN")))), host: NString(Some(Quoted(Quoted("MIT.EDU")))) }], bcc: [], in_reply_to: NString(None), message_id: NString(Some(Quoted(Quoted("<B27397-0100000@cac.washington.edu>")))) }), Body(Single { body: Body { basic: BasicFields { parameter_list: [(Quoted(Quoted("CHARSET")), Quoted(Quoted("US-ASCII")))], id: NString(None), description: NString(None), content_transfer_encoding: Quoted(Quoted("7BIT")), size: 3028 }, specific: Text { subtype: Quoted(Quoted("PLAIN")), number_of_lines: 92 } }, extension: None })] })
// * 12 FETCH (FLAGS (\Seen) INTERNALDATE "17-Jul-1996 02:44:25 -0700" RFC822.SIZE 4286 ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US")("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") BODY ("TEXT" "PLAIN" ("CHARSET" "US-ASCII") NIL NIL "7BIT" 3028 92))

// a003 OK FETCH completed
Status(Ok { tag: Some(Tag("a003")), code: None, text: Text("FETCH completed") })
// a003 OK FETCH completed

// a004 fetch 12 body[header]
Command { tag: Tag("a004"), body: Fetch { sequence_set: SequenceSet([Single(Value(12))]), attributes: FetchAttributes([BodyExt { section: Some(Header(None)), partial: None, peek: false }]), uid: false } }
// a004 FETCH 12 BODY[HEADER]

// a004 OK FETCH completed
Status(Ok { tag: Some(Tag("a004")), code: None, text: Text("FETCH completed") })
// a004 OK FETCH completed

// a005 store 12 +flags \deleted
Command { tag: Tag("a005"), body: Store { sequence_set: SequenceSet([Single(Value(12))]), kind: Add, response: Answer, flags: [Deleted], uid: false } }
// a005 STORE 12 +FLAGS (\Deleted)

// * 12 FETCH (FLAGS (\Seen \Deleted))
Data(Fetch { seq_or_uid: 12, attributes: [Flags([Seen, Deleted])] })
// * 12 FETCH (FLAGS (\Seen \Deleted))

// a005 OK +FLAGS completed
Status(Ok { tag: Some(Tag("a005")), code: None, text: Text("+FLAGS completed") })
// a005 OK +FLAGS completed

// a006 logout
Command { tag: Tag("a006"), body: Logout }
// a006 LOGOUT

// * BYE IMAP4rev1 server terminating connection
Status(Bye { code: None, text: Text("IMAP4rev1 server terminating connection") })
// * BYE IMAP4rev1 server terminating connection

// a006 OK LOGOUT completed
Status(Ok { tag: Some(Tag("a006")), code: None, text: Text("LOGOUT completed") })
// a006 OK LOGOUT completed
```

## A Note on IMAP literals

IMAP literals make separating the parsing logic from the application logic difficult.
When a parser recognizes a literal (e.g. "{42}"), a so-called continuation response (`+ ...`) must be sent.
Otherwise, the client or server will not send more data, and a parser would always return `Incomplete(42)`.

A possible solution is to implement a framing codec first.
This strategy is motivated by the IMAP RFC:

```
The protocol receiver of an IMAP4rev1 client or server is either reading a line,
or is reading a sequence of octets with a known count followed by a line.
```

The framing codec can be implemented like this ...

```rust
loop {
    line = read_line()
    if line.has_literal() {
        literal = read_literal(amount)
    }
}
```

... and variants of this procedure are provided in the [parse_command] example and the [demo server].

# License

This crate is dual-licensed under Apache 2.0 and MIT terms.

[IMAP4rev1]: https://tools.ietf.org/html/rfc3501
[imap-types]: https://github.com/duesee/imap-codec/imap-types
[formal syntax]: https://tools.ietf.org/html/rfc3501#section-9
[documentation]: https://docs.rs/imap-codec/latest/imap_codec/
[cargo fuzz]: https://github.com/rust-fuzz/cargo-fuzz
[demo client]: https://github.com/duesee/imap-codec/tree/main/assets/demos/tokio_client
[demo server]: https://github.com/duesee/imap-codec/tree/main/assets/demos/tokio_server
[parse_command]: https://github.com/duesee/imap-codec/blob/main/examples/parse_command.rs

# Thanks

Thanks to the [NLnet Foundation](https://nlnet.nl/) for supporting imap-codec through their [NGI Assure](https://nlnet.nl/assure/) program!

<div align="right">
    <img height="100px" src="https://user-images.githubusercontent.com/8997731/215262095-ab12d43a-ca8a-4d44-b79b-7e99ab91ca01.png"/>
    <img height="100px" src="https://user-images.githubusercontent.com/8997731/221422192-60d28ed4-10bb-441e-957d-93af58166707.png"/>
    <img height="100px" src="https://user-images.githubusercontent.com/8997731/215262235-0db02da9-7c6c-498e-a3d2-7ea7901637bf.png"/>
</div>
