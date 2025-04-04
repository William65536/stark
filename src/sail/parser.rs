// STARK, a system for computer augmented design.

// SPDX-FileCopyrightText: © 2021 Matthew Rothlisberger
// SPDX-License-Identifier: AGPL-3.0-only

// STARK is licensed under the terms of the GNU Affero General Public
// License version 3. See the top-level LICENSES directory for the
// license text.

// Find full copyright information in the top-level COPYRIGHT file.

// <>

// src/sail/parser.rs

// Recursive descent parser which converts string slices into Sail
// objects, usually for evaluation.

// <>

// TODO: move to using &mut for the Region instead of *mut

use super::{SlErrCode, Stab, core::*, memmgt};

use std::iter;
use std::str;

// struct Parser {
//     chars: iter::Peekable<str::Bytes<'static>>,
//     acc: Vec<u8>,
// }

/// Parses a textual Sail expression into a structure of Sail objects
pub fn parse(
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
    code: &str,
    file: bool,
) -> Result<SlHndl, SlErrCode> {
    // Accumulator for collecting string values
    let mut acc: Vec<u8> = Vec::new();

    let chars = if file {
        String::from("(do ") + code + ")"
    } else {
        code.to_string()
    };

    let mut cit = chars.bytes().peekable();

    let val = read_value(&mut cit, &mut acc, reg, tbl)?;

    Ok(val)
}

// pub fn parse_bytes(tbl: *mut SlHead, code: &[u8]) -> Result<*mut SlHead, SlErrCode> {
//     let mut acc: Vec<u8> = Vec::new();
//     let mut chars = code.iter().peekable();

//     read_value(&mut chars, &mut acc, tbl, false)
// }

/// Returns the head of a Sail object structure representing a single
/// item parsed from the input stream
///
/// This is a recursive descent parser; the appropriate reader can
/// almost always be deduced from the first character
fn read_value(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let value;

    let mut c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    while c.is_ascii_whitespace() || c == b';' {
        if c == b';' {
            while *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?) != b'\n' {
                chars.next();
            }
        }
        chars.next();
        c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    }

    match c {
        b'\'' => {
            chars.next();
            value = read_quote(chars, acc, reg, tbl)?;
        }
        b'(' => {
            chars.next();
            value = read_list(chars, acc, reg, tbl)?;
        }
        b'[' => {
            chars.next();
            value = read_vec(chars, acc, reg, tbl)?;
        }
        b'{' => {
            chars.next();
            value = read_map(chars, acc, reg, tbl)?;
        }
        b':' => {
            chars.next();
            value = read_spec_sym(chars, acc, reg, tbl, SymbolMode::Keyword)?;
            acc.clear();
        }
        b'$' => {
            chars.next();
            value = read_spec_sym(chars, acc, reg, tbl, SymbolMode::Type)?;
            acc.clear();
        }
        b'@' => {
            chars.next();
            value = read_spec_sym(chars, acc, reg, tbl, SymbolMode::Module)?;
            acc.clear();
        }
        b'"' => {
            chars.next();
            value = read_string(chars, acc, reg, tbl)?;
            acc.clear();
        }
        b'#' => {
            chars.next();
            value = read_special(chars, acc, reg, tbl)?;
            acc.clear();
        }
        b'+' | b'-' => {
            acc.push(chars.next().unwrap());
            if chars
                .peek()
                .ok_or(SlErrCode::ParseUnexpectedEnd)?
                .is_ascii_digit()
            {
                value = read_number(chars, acc, reg, tbl)?;
            } else {
                value = read_symbol(chars, acc, reg, tbl)?;
            }
            acc.clear();
        }
        b'*' | b'/' | b'<' | b'=' | b'>' | b'_' => {
            value = read_symbol(chars, acc, reg, tbl)?;
            acc.clear();
        }
        _ if c.is_ascii_alphabetic() => {
            value = read_symbol(chars, acc, reg, tbl)?;
            acc.clear();
        }
        _ if c.is_ascii_digit() => {
            value = read_number(chars, acc, reg, tbl)?;
            acc.clear();
        }
        _ => {
            return Err(SlErrCode::ParseInvalidChar);
        }
    }
    Ok(value)
}

/// Reads a quoted expression off the input stream, into the
/// appropriate object structure
fn read_quote(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let start = sym_init(reg, super::SP_QUOTE.0);
    let head = ref_init(reg, start.clone());

    let end = read_value(chars, acc, reg, tbl)?;
    unsafe {
        inc_refc(end.get_raw());
        set_next_list_elt_unsafe_unchecked(start, end);
    }

    Ok(head)
}

/// Reads a list of values from the input stream and creates a
/// corresponding list of Sail objects
fn read_list(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let head = ref_make(reg);

    let mut c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    if c == b')' {
        chars.next();
        return Ok(head);
    }

    let mut count = 0;
    let mut tail = head.clone();

    while c != b')' {
        match c {
            b';' => while chars.next().unwrap_or(b'\n') != b'\n' {},
            _ if c.is_ascii_whitespace() => {
                chars.next();
            }
            _ => {
                // append to the list tail
                tail = {
                    let next = read_value(chars, acc, reg, tbl)?;
                    unsafe {
                        inc_refc(next.get_raw());
                        if count < 1 {
                            write_ptr_unsafe_unchecked(tail, 0, next.clone())
                        } else {
                            set_next_list_elt_unsafe_unchecked(tail, next.clone())
                        }
                    }
                    next
                };

                count += 1;
            }
        }

        c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    }

    chars.next();
    Ok(head)
}

// TODO: lists may need to be evaluated even if they appear in a vec or map
// TODO: tighter parser-evaluator integration likely necessary for this & symbols

/// Reads a vector from the input stream and creates the corresponding
/// Sail object
fn read_vec(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let mut tvc = vec![];
    let mut c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    while c != b']' {
        match c {
            b';' => while chars.next().unwrap_or(b'\n') != b'\n' {},
            _ if c.is_ascii_whitespace() => {
                chars.next();
            }
            _ => tvc.push(read_value(chars, acc, reg, tbl)?),
        }
        c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    }
    let vec = stdvec_init(reg, &tvc.into_boxed_slice());
    chars.next();
    Ok(vec)
}

/// Reads an associative map from the input stream and creates the
/// corresponding Sail object
fn read_map(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let map = hashvec_make(reg, 16);
    let mut c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    while c != b'}' {
        match c {
            b';' => while chars.next().unwrap_or(b'\n') != b'\n' {},
            _ if c.is_ascii_whitespace() => {
                chars.next();
            }
            _ => hash_map_insert(
                reg,
                map.clone(),
                read_value(chars, acc, reg, tbl)?,
                read_value(chars, acc, reg, tbl)?,
            ),
        }
        c = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    }
    chars.next();
    Ok(map)
}

/// Reads a basic symbol from the input stream and creates its Sail
/// object
fn read_symbol(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    while {
        let peek = chars.peek().unwrap_or(&b' ');
        match peek {
            b')' | b']' | b'}' => false,
            _ if peek.is_ascii_whitespace() => false,
            _ => true,
        }
    } {
        let next = chars.next().unwrap();
        match next {
            b'!' | b'*' | b'+' | b'-' | b'/' | b'<' | b'=' | b'>' | b'?' | b'_' => acc.push(next),
            _ if next.is_ascii_alphanumeric() => acc.push(next),
            _ => {
                return Err(SlErrCode::ParseInvalidChar);
            }
        }
    }

    let sym = sym_init(reg, tbl.get_id(acc.as_slice()));

    Ok(sym)
}

/// Reads a specialized symbol from the input stream and creates its
/// Sail object
fn read_spec_sym(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    tbl: &mut Stab,
    mode: SymbolMode,
) -> Result<SlHndl, SlErrCode> {
    while {
        let peek = chars.peek().unwrap_or(&b' ');
        match peek {
            b')' | b']' | b'}' => false,
            _ if peek.is_ascii_whitespace() => false,
            _ => true,
        }
    } {
        let next = chars.next().unwrap();
        match next {
            b'-' | b'_' => acc.push(next),
            _ if next.is_ascii_alphanumeric() => acc.push(next),
            _ => {
                return Err(SlErrCode::ParseInvalidChar);
            }
        }
    }

    if acc.is_empty() {
        return Err(SlErrCode::ParseUnexpectedEnd);
    }

    let sym = sym_init(reg, super::modeize_sym(tbl.get_id(acc.as_slice()), mode));

    Ok(sym)
}

/// Reads a string from the input stream and creates its Sail object
fn read_string(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    _tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    let mut next = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    while next != b'"' {
        acc.push(chars.next().unwrap());
        next = *(chars.peek().ok_or(SlErrCode::ParseUnexpectedEnd)?);
    }

    chars.next();

    let string = string_init(
        reg,
        match str::from_utf8(acc) {
            Ok(s) => s,
            _ => return Err(SlErrCode::ParseInvalidString),
        },
    );

    Ok(string)
}

/// Reads a number from the input stream and creates its Sail object
fn read_number(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    _tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    while {
        let peek = chars.peek().unwrap_or(&b' ');
        match peek {
            b')' | b']' | b'}' => false,
            _ if peek.is_ascii_whitespace() => false,
            _ => true,
        }
    } {
        let next = chars.next().unwrap();
        match next {
            b'+' | b'-' | b'_' | b'.' => acc.push(next),
            _ if next.is_ascii_alphanumeric() => acc.push(next),
            _ => {
                return Err(SlErrCode::ParseInvalidChar);
            }
        }
    }
    process_num(unsafe { str::from_utf8_unchecked(acc) }, reg)
}

/// Reads a special item from the input stream and creates a Sail
/// object if appropriate
fn read_special(
    chars: &mut iter::Peekable<str::Bytes>,
    acc: &mut Vec<u8>,
    reg: *mut memmgt::Region,
    _tbl: &mut Stab,
) -> Result<SlHndl, SlErrCode> {
    while {
        let peek = chars.peek().unwrap_or(&b' ');
        match peek {
            b')' | b']' | b'}' => false,
            _ if peek.is_ascii_whitespace() => false,
            _ => true,
        }
    } {
        let next = chars.next().unwrap();
        match next {
            b'_' => acc.push(next),
            _ if next.is_ascii_alphanumeric() => acc.push(next),
            _ => {
                return Err(SlErrCode::ParseInvalidChar);
            }
        }
    }

    if acc.is_empty() {
        return Err(SlErrCode::ParseUnexpectedEnd);
    }

    if acc[0].eq_ignore_ascii_case(&b't') && acc.len() == 1 {
        Ok(bool_init(reg, true))
    } else if acc[0].eq_ignore_ascii_case(&b'f') && acc.len() == 1 {
        Ok(bool_init(reg, false))
    } else {
        Err(SlErrCode::ParseBadSpecial)
    }
}

/// Parses a number and creates an object according to its textual
/// representation
fn process_num(slice: &str, reg: *mut memmgt::Region) -> Result<SlHndl, SlErrCode> {
    if let Ok(n) = slice.parse::<i64>() {
        Ok(i64_init(reg, n))
    } else if let Ok(n) = slice.parse::<f64>() {
        Ok(f64_init(reg, n))
    } else {
        Err(SlErrCode::ParseInvalidNum)
    }
}
