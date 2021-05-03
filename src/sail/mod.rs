// STARK, a system for computer augmented design.
// Copyright (C) 2021 Matthew Rothlisberger

// STARK is free software: you can redistribute it and / or modify it
// under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.

// STARK is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public
// License along with STARK (in the LICENSE file). If not, see
// <https://www.gnu.org/licenses/>.

// Find full copyright information in the top level COPYRIGHT file.

// <>

// src/sail/mod.rs

// Sail is a Lisp dialect built specifically for STARK. There is a
// parser, interpreter, REPL, and small "standard library". Much of
// STARK's function will be described in Sail.

// <>

//! The Structured Augmentation Interchange Language
//! A custom Lisp dialect for writing STARK

use std::convert::TryFrom;
use std::fmt;
use std::mem;
use std::ptr;

#[macro_use]
mod core;
pub use self::core::*;

pub mod eval;
pub mod memmgt;
pub mod parser;
pub mod queue;

#[derive(Debug)]
#[repr(u16)]
pub enum SlErrCode {
    ErrorUnknown = 0,
    ParseUnexpectedEnd,
    ParseBadSpecial,
    ParseInvalidChar,
    ParseInvalidString,
    ParseInvalidNum,
    FileCouldNotRead,
}

impl TryFrom<u16> for SlErrCode {
    type Error = ();

    #[inline(always)]
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        use SlErrCode::*;
        match v {
            x if x == ErrorUnknown as u16 => Ok(ErrorUnknown),
            x if x == ParseUnexpectedEnd as u16 => Ok(ParseUnexpectedEnd),
            x if x == ParseBadSpecial as u16 => Ok(ParseBadSpecial),
            x if x == ParseInvalidChar as u16 => Ok(ParseInvalidChar),
            x if x == ParseInvalidString as u16 => Ok(ParseInvalidString),
            x if x == ParseInvalidNum as u16 => Ok(ParseInvalidNum),
            x if x == FileCouldNotRead as u16 => Ok(FileCouldNotRead),
            _ => Err(()),
        }
    }
}

#[inline(always)]
fn errcode_make(reg: *mut memmgt::Region) -> *mut SlHead {
    unsafe {
        let ptr = memmgt::alloc(reg, 2, Cfg::B2Err as u8);
        write_field_unchecked::<u16>(ptr, 0, 0);
        ptr
    }
}

#[inline(always)]
fn errcode_init(reg: *mut memmgt::Region, err: SlErrCode) -> *mut SlHead {
    unsafe {
        let ptr = memmgt::alloc(reg, 2, Cfg::B2Err as u8);
        write_field_unchecked::<u16>(ptr, 0, err as u16);
        ptr
    }
}

#[inline(always)]
fn errcode_set(loc: *mut SlHead, err: SlErrCode) {
    coretypck!(loc ; ErrCode);
    core_write_field(loc, 0, err as u16);
}

#[inline(always)]
fn errcode_get(loc: *mut SlHead) -> SlErrCode {
    coretypck!(loc ; ErrCode);
    SlErrCode::try_from(core_read_field::<u16>(loc, 0)).unwrap()
}

/// TODO: remember types that are parents / children of others
/// TODO: types with children cannot be a self type
/// TODO: these items must be added to the symtab and env on every run
/// TODO: remember to have a bitvec type
/// TODO: use a script to automatically generate a Rust "env" file
macro_rules! incl_symbols {
    ( $array:ident : $( $id:literal $name:ident $strng:literal $mode:ident );+ $size:literal ) => {
        $(
            pub const $name: (u32, &str) = (modeize_sym($id, SymbolMode::$mode), $strng);
        )+
            const $array: [&str; $size] = [$($strng),+];
    };
}

incl_symbols! {
    SYM_ARRAY:
    0  T_T           "t"       Type;
    1  T_NIL         "nil"     Type;
    2  T_BOOL        "bool"    Type;
    3  T_U8          "u8"      Type;
    4  T_I8          "i8"      Type;
    5  T_U16         "u16"     Type;
    6  T_I16         "i16"     Type;
    7  T_U32         "u32"     Type;
    8  T_I32         "i32"     Type;
    9  T_U64         "u64"     Type;
    10 T_I64         "i64"     Type;
    11 T_U128        "u128"    Type;
    12 T_I128        "i128"    Type;
    13 T_F32         "f32"     Type;
    14 T_F64         "f64"     Type;
    15 T_SYMBOL      "symbol"  Type;
    16 T_REF         "ref"     Type;
    17 T_VECTOR      "vector"  Type;
    18 T_STDVEC      "stdvec"  Type;
    19 T_STRING      "string"  Type;
    20 T_HASHVEC     "hashvec" Type;
    21 T_ANYVEC      "anyvec"  Type;
    22 T_MAP         "map"     Type;
    23 T_ALISMAP     "alismap" Type;
    24 T_HASHMAP     "hashmap" Type;
    25 T_PROC        "proc"    Type;
    26 T_PROC_LAMBDA "sail-fn" Type;
    27 T_PROC_NATIVE "rust-fn" Type;
    28 T_ERR         "err"     Type;
    29 T_QUEUE_TX    "q-tx"    Type;
    30 T_QUEUE_RX    "q-rx"    Type;
    31 SP_DEF        "def"     Basic;
    32 SP_DO         "do"      Basic;
    33 SP_EVAL       "eval"    Basic;
    34 SP_FN         "fn"      Basic;
    35 SP_IF         "if"      Basic;
    36 SP_QUOTE      "quote"   Basic;
    37 SP_SET        "set"     Basic;
    38 SP_WHILE      "while"   Basic;
    39 S_MR_SEND     "mr-send" Basic;
    40 S_MR_RECV     "mr-recv" Basic;
    41 S_CM_SEND     "cm-send" Basic;
    42 S_CM_RECV     "cm-recv" Basic;
    43 S_CR_SEND     "cr-send" Basic;
    44 S_CR_RECV     "cr-recv" Basic;
    45 S_MAIN        "main"    Basic;
    46 S_WINDOW      "window"  Basic;
    47 S_RNDR        "rndr"    Basic;
    48 S_ENGINE      "engine"  Basic;
    49 S_T_INTERN    "%true"   Basic;
    50 K_CX_DESTR    "cx-dstr" Keyword;
    51 K_CX_RESIZ    "cx-resz" Keyword;
    52 K_CX_CLICK    "cx-clck" Keyword;
    53 K_CX_REDRW    "cx-rdrw" Keyword;
    54 K_CX_SHELL    "cx-shel" Keyword
    55
}

/// Set a symbol to one of the four symbol modes
const fn modeize_sym(sym: u32, mode: SymbolMode) -> u32 {
    (sym & 0x3FFFFFFF) + ((mode as u32) << 30)
}

/// Returns a symbol set to the default, basic mode
const fn demodes_sym(sym: u32) -> u32 {
    sym & 0x3FFFFFFF
}

/// Get the mode of a symbol
const fn mode_of_sym(sym: u32) -> SymbolMode {
    unsafe { mem::transmute::<u8, SymbolMode>((sym >> 30) as u8) }
}

// TODO: MINIMIZE the use of *pub* and *unsafe* functions

fn get_self_type(loc: *mut SlHead) -> u32 {
    if nil_p(loc) {
        return T_NIL.0;
    }
    use Cfg::*;
    match get_cfg_spec(loc) {
        B0BoolF | B0BoolT => T_BOOL.0,
        B1U8 => T_U8.0,
        B1I8 => T_I8.0,
        B2U16 => T_U16.0,
        B2I16 => T_I16.0,
        B4U32 => T_U32.0,
        B4I32 => T_I32.0,
        B4F32 => T_F32.0,
        B4Sym => T_SYMBOL.0,
        B8U64 => T_U64.0,
        B8I64 => T_I64.0,
        B8F64 => T_F64.0,
        B8Ptr => T_REF.0,
        B16U128 => T_U128.0,
        B16I128 => T_I128.0,
        VecStd => T_STDVEC.0,
        VecStr => T_STRING.0,
        VecHash => T_HASHVEC.0,
        VecAny => T_ANYVEC.0,
        ProcLambda => T_PROC_LAMBDA.0,
        ProcNative => T_PROC_NATIVE.0,
        _ => {
            assert!(self_type_p(loc));
            unsafe { ptr::read_unaligned((loc as *mut u8).add(HEAD_LEN as usize) as *const u32) }
        }
    }
}

fn get_pred_type(loc: *mut SlHead) -> u32 {
    if nil_p(loc) {
        T_NIL.0
    } else if pred_type_p(loc) {
        unsafe {
            ptr::read_unaligned((loc as *const u8).add(if !self_type_p(loc) {
                HEAD_LEN
            } else {
                HEAD_LEN + SYMBOL_LEN
            } as usize) as *const u32)
        }
    } else {
        // return $t, true for all types
        T_T.0
    }
}

// /// Returns the size of a valid Sail value
// fn get_size(
//     reg: *mut memmgt::Region,
//     tbl: *mut SlHead,
//     env: *mut SlHead,
//     loc: *mut SlHead,
// ) -> usize {
//     use BaseSize::*;
//     match get_base_size(loc) {
//         B0 => 0,
//         B1 => 1,
//         B2 => 2,
//         B4 => 4,
//         B8 => 8,
//         B16 => 16,
//         _ => match core_type(loc) {
//             Some(_) => core_size(loc),
//             None => {
//                 let entry = env_lookup_by_id(reg, env, get_self_type(loc));
//                 let entry_type = get_self_type(entry);

//                 if entry_type == T_U64.0 {
//                     return u64_get(entry) as usize;
//                 } else if entry_type == T_PROC_LAMBDA.0 || entry_type == T_PROC_NATIVE.0 {
//                     u64_get(apply(reg, tbl, env, entry, loc).unwrap()) as usize
//                 } else {
//                     panic!("wrong type in type entry")
//                 }
//             }
//         },
//     }
// }

fn set_self_type(loc: *mut SlHead, typ: u32) {
    assert!(self_type_p(loc));
    unsafe { ptr::write_unaligned((loc as *mut u8).add(HEAD_LEN as usize) as *mut u32, typ) }
}

fn set_pred_type(loc: *mut SlHead, typ: u32) {
    assert!(pred_type_p(loc));
    unsafe {
        ptr::write_unaligned(
            (loc as *mut u8).add(if self_type_p(loc) {
                HEAD_LEN + SYMBOL_LEN
            } else {
                HEAD_LEN
            } as usize) as *mut u32,
            typ,
        )
    }
}

// /// TODO: eliminate as much write_unaligned as possible
// fn write_field<T: SizedBase>(
//     reg: *mut memmgt::Region,
//     tbl: *mut SlHead,
//     env: *mut SlHead,
//     loc: *mut SlHead,
//     offset: usize,
//     src: T,
// ) {
//     unsafe {
//         let dst = value_ptr(loc).add(offset) as *mut T;
//         assert!(offset + std::mem::size_of::<T>() <= get_size(reg, tbl, env, loc));
//         ptr::write_unaligned(dst, src)
//     }
// }

// /// TODO: eliminate as much read_unaligned as possible
// fn read_field<T: SizedBase>(
//     reg: *mut memmgt::Region,
//     tbl: *mut SlHead,
//     env: *mut SlHead,
//     loc: *mut SlHead,
//     offset: usize,
// ) -> T {
//     unsafe {
//         let src = value_ptr(loc).add(offset) as *mut T;
//         assert!(offset + std::mem::size_of::<T>() <= get_size(reg, tbl, env, loc));
//         ptr::read_unaligned(src)
//     }
// }

union _SlSend {
    ptr: *mut SlHead,
    num: usize,
}

/// Bundles together a value and associated symbol table for display
pub struct SlContextVal {
    tbl: *mut SlHead,
    val: *mut SlHead,
}

pub fn context(tbl: *mut SlHead, val: *mut SlHead) -> SlContextVal {
    SlContextVal { tbl, val }
}

// TODO: just push characters into a byte vector (string) for display
impl fmt::Display for SlContextVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let table = self.tbl;
        let value = self.val;

        use CoreType::*;
        match core_type(value) {
            Some(t) => match t {
                Nil => write!(f, "()"),
                Bool => write!(f, "{}", if bool_get(value) { "#T" } else { "#F" }),
                I64 => write!(f, "{}", i64_get(value)),
                F64 => write!(f, "{}", f64_get(value)),
                F32 => write!(f, "{}", f32_get(value)),
                ErrCode => write!(f, "<err: {:?}>", errcode_get(value)),
                Symbol => {
                    let full_id = sym_get_id(value);
                    match mode_of_sym(full_id) {
                        SymbolMode::Basic => {
                            write!(f, "{}", string_get(sym_tab_lookup_id_num(table, full_id)))
                        }
                        SymbolMode::Keyword => {
                            write!(
                                f,
                                ":{}",
                                string_get(sym_tab_lookup_id_num(table, demodes_sym(full_id)))
                            )
                        }
                        SymbolMode::Module => {
                            write!(
                                f,
                                "@{}",
                                string_get(sym_tab_lookup_id_num(table, demodes_sym(full_id)))
                            )
                        }
                        SymbolMode::Type => {
                            write!(
                                f,
                                "${}",
                                string_get(sym_tab_lookup_id_num(table, demodes_sym(full_id)))
                            )
                        }
                    }
                }
                Ref => {
                    write!(f, "(").unwrap();
                    let mut elt = ref_get(value);
                    while !nil_p(elt) {
                        write!(f, "{}", context(table, elt).to_string()).unwrap();
                        elt = get_next_list_elt(elt);
                        if !nil_p(elt) {
                            write!(f, " ").unwrap();
                        }
                    }
                    write!(f, ")")
                }
                VecStd => {
                    write!(f, "[").unwrap();
                    let len = stdvec_get_len(value);
                    for idx in 0..len {
                        write!(f, "{}", context(table, stdvec_idx(value, idx)).to_string())
                            .unwrap();
                        if idx < len - 1 {
                            write!(f, " ").unwrap();
                        }
                    }
                    write!(f, "]")
                }
                VecStr => write!(f, "\"{}\"", string_get(value)),
                VecHash => {
                    // TODO: function to access all map pairs somehow
                    write!(f, "{{").unwrap();
                    // visit every position and traverse each entry list as below
                    let size = hashvec_get_size(value);
                    let mut fst = true;
                    for idx in 0..size {
                        let mut pos =
                            core_read_field(value, 4 + 4 + (PTR_LEN as usize * idx as usize));
                        while !nil_p(pos) {
                            if !fst {
                                write!(f, " ").unwrap()
                            }
                            write!(f, "{} ", context(table, ref_get(pos)).to_string()).unwrap();
                            write!(
                                f,
                                "{}",
                                context(table, get_next_list_elt(ref_get(pos))).to_string()
                            )
                            .unwrap();
                            pos = get_next_list_elt(pos);
                            fst = false;
                        }
                    }
                    write!(f, "}}")
                }
                ProcLambda | ProcNative => write!(f, "<$proc>"),
                _ => write!(f, "<@core/$other>"),
            },
            None => write!(f, "<$other>"),
        }
    }
}

/// Accepts an input stream and runs a read - evaluate - print loop perpetually
pub fn repl(stream_in: std::io::Stdin) {
    // TODO: Consider stack-like environment per function
    // TODO: Start to think about namespaces etc

    let region = unsafe { memmgt::acquire_mem_region(100000) };

    // Create persistent environment and symbol table
    let (tbl, env) = prep_environment(region);

    // Load standard / base definitions into environment and symbol table
    environment_setup(region, tbl, env);

    let mut stack = eval::EvalStack::new(10000);

    let sigil = 1 as *mut SlHead;

    let mut ret_slot = sigil;
    let ret_addr: *mut *mut SlHead = &mut ret_slot;

    loop {
        let mut input = String::new();
        stream_in.read_line(&mut input).expect("Failure");

        let expr = match parser::parse(region, tbl, &input) {
            Ok(out) => out,
            Err(err) => {
                println!("{:?}", err);
                continue;
            }
        };

        stack.start(ret_addr, env, expr);

        while ret_slot == sigil {
            stack.iter_once(region, tbl);
        }

        println!("{}\n", context(tbl, ret_slot).to_string());

        ret_slot = sigil;
    }
}

pub fn run_file(filename: &str) -> Result<String, SlErrCode> {
    let file = match std::fs::read_to_string(filename) {
        Ok(s) => s,
        Err(_) => return Err(SlErrCode::FileCouldNotRead),
    };
    interpret(&file)
}

/// Interprets a Sail expression, returning the result
pub fn interpret(code: &str) -> Result<String, SlErrCode> {
    let region = unsafe { memmgt::acquire_mem_region(1000000) };

    let (tbl, env) = prep_environment(region);

    environment_setup(region, tbl, env);

    let expr = parser::parse(region, tbl, code)?;
    let result = eval::eval(region, tbl, env, expr);

    Ok(context(tbl, result).to_string())
}

/// TODO: make it easier to add native functions to the environment
pub fn environment_setup(reg: *mut memmgt::Region, tbl: *mut SlHead, env: *mut SlHead) {
    for s in SYM_ARRAY.iter() {
        sym_tab_get_id(reg, tbl, s);
    }

    let true_intern = bool_init(reg, true);
    env_layer_ins_by_id(reg, env, S_T_INTERN.0, true_intern);

    insert_native_procs(reg, tbl, env, &NATFNS[..]);
}

/// TODO: intended to be temporary; still relies on some "magic values"
pub fn insert_native_proc(
    reg: *mut memmgt::Region,
    tbl: *mut SlHead,
    env: *mut SlHead,
    name: &str,
    func: NativeFn,
    argct: u16,
) {
    let proc_id = sym_init(reg, sym_tab_get_id(reg, tbl, name));

    let proc_fn = proc_native_make(reg, argct);
    proc_native_set_body(proc_fn, func);

    env_layer_ins_entry(reg, env, proc_id, proc_fn);
}

fn insert_native_procs(
    reg: *mut memmgt::Region,
    tbl: *mut SlHead,
    env: *mut SlHead,
    fns: &[(&str, NativeFn, u16)],
) {
    for entry in fns {
        let proc_id = sym_init(reg, sym_tab_get_id(reg, tbl, entry.0));

        let proc_fn = proc_native_make(reg, entry.2);
        proc_native_set_body(proc_fn, entry.1);

        env_layer_ins_entry(reg, env, proc_id, proc_fn);
    }
}

/// TODO: type checks and variable length arglists for native functions
/// TODO: generate these functions somehow else if macros won't cut it
macro_rules! sail_fn {
    ( $reg:ident $tbl:ident $env:ident ; $fnct:literal $array:ident
      $( $name:literal $fn_name:ident $argct:literal [ $($args:ident),* ] $body:block )+
    ) => {
        $(
            fn $fn_name (
                _reg: *mut memmgt::Region,
                _tbl: *mut SlHead,
                _env: *mut SlHead,
                _args: &[*mut SlHead]
            ) -> *mut SlHead {
                let $reg = _reg;
                let $tbl = _tbl;
                let $env = _env;

                let mut _ind = 0;
                $(
                    let $args = _args[_ind];
                    _ind += 1;
                )*

                $body
            }
        )+

            const $array: [(&str, NativeFn, u16); $fnct] = [$(($name, $fn_name, $argct)),+];
    };
}

// TODO: native functions MUST be fully safe to use
sail_fn! {
    _reg _tbl _env ;

    17 NATFNS

    "+" add 2 [fst, snd] {
        let out = i64_make(_reg);
        let result = i64_get(fst) + i64_get(snd);
        i64_set(out, result);
        return out;
    }

    "-" sub 2 [fst, snd] {
        let out = i64_make(_reg);
        let result = i64_get(fst) - i64_get(snd);
        i64_set(out, result);
        return out;
    }

    "mod" modulus 2 [fst, snd] {
        let out = i64_make(_reg);
        let result = i64_get(fst) % i64_get(snd);
        i64_set(out, result);
        return out;
    }

    "=" num_eq 2 [fst, snd] {
        // let out = init_bool(reg);
        let result = i64_get(fst) == i64_get(snd);
        if result {
            env_lookup_by_id(_env, S_T_INTERN.0)
        } else {
            nil()
        }
        // bool_set(out, result);
        // return out;
    }

    "eq" sym_eq 2 [fst, snd] {
        // let out = init_bool(reg);
        let result = core_eq(fst, snd);
        if result {
            env_lookup_by_id(_env, S_T_INTERN.0)
        } else {
            nil()
        }
        // bool_set(out, result);
        // return out;
    }

    "not" not 1 [val] {
        // let out = init_bool(reg);
        if !truthy(val) {
            // bool_set(out, false)
            env_lookup_by_id(_env, S_T_INTERN.0)
        } else {
            // bool_set(out, true)
            nil()
        }
        // return out;
    }

    "qtx" qtx 2 [sender, item] {
        queue::queue_tx(sender, item);

        // let out = init_bool(reg);
        // bool_set(out, true);
        // return out;

        return nil();
    }

    "qrx" qrx 2 [receiver] {
        return queue::queue_rx(receiver);
    }

    "vec-f32-make" vec_f32_make 0 [] {
        let vec = unsafe { memmgt::alloc(_reg, vec_size(12, 4, 8), Cfg::VecAny as u8) };
        unsafe {
            write_field_unchecked(vec, 0, T_F32.0);
            write_field_unchecked(vec, 4, 8 as u32);
        }
        core_write_field(vec, 8, 0 as u32);
        return vec;
    }

    "vec-f32-push" vec_f32_push 2 [target, value] {
        coretypck!(target ; VecAny);
        coretypck!(value ; F32);
        assert_eq!(core_read_field::<u32>(target, 0), T_F32.0);

        let cap = core_read_field::<u32>(target, 4);
        let len = core_read_field::<u32>(target, 8);

        if len < cap {
            core_write_field(target, 8, len + 1);
            core_write_field(target, (4 * len as usize) + 12, f32_get(value));
        }

        return target;
    }

    "vec-f32-get" vec_f32_get 2 [target, idx] {
        coretypck!(target ; VecAny);
        coretypck!(idx ; I64);
        assert_eq!(core_read_field::<u32>(target, 0), T_F32.0);

        let len = core_read_field::<u32>(target, 8);
        let idx = i64_get(idx) as u32;

        return if idx < len {
            f32_init(_reg, core_read_field::<f32>(target, (4 * idx as usize) + 12))
        } else {
            panic!("invalid index")
        };
    }

    "vec-f32-set" vec_f32_set 3 [target, idx, val] {
        coretypck!(target ; VecAny);
        coretypck!(idx ; I64);
        coretypck!(val; F32);
        assert_eq!(core_read_field::<u32>(target, 0), T_F32.0);

        let len = core_read_field::<u32>(target, 8);
        let idx = i64_get(idx) as u32;

        if idx < len {
            core_write_field::<f32>(target, (4 * idx as usize) + 12, f32_get(val));
        } else {
            panic!("invalid index")
        }

        return target;
    }

    "as-f32" as_f32 1 [val] {
        return f32_init(_reg, f64_get(val) as f32);
    }

    "print" print 1 [arg] {
        println!("{}", context(_tbl, arg).to_string());
        return nil();
    }

    "dbg" dbg 1 [arg] {
        println!("{}", context(_tbl, arg).to_string());
        return arg;
    }

    "printenv" printenv 0 [] {
        println!("{}", context(_tbl, _env).to_string());
        return nil();
    }

    "parse" parse 1 [strin] {
        coretypck!(strin ; VecStr);
        let strsl = string_get(strin);

        return match parser::parse(_reg, _tbl, strsl) {
            Ok(head) => head,
            Err(err) => errcode_init(_reg, err),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns() {
        let exp = String::from("42");
        assert_eq!(exp, interpret(&exp).unwrap());
    }

    #[test]
    fn adds() {
        let exp = String::from("(+ 2 2)");
        assert_eq!("4", interpret(&exp).unwrap());
    }

    #[test]
    fn parses() {
        let (reg, tbl) = unsafe {
            let reg = memmgt::acquire_mem_region(10000);
            (reg, prep_environment(reg).0)
        };

        let exp = String::from("(+ (() 42 (e) #T) #F 2.1 e)");
        let val = parser::parse(reg, tbl, &exp).unwrap();
        let out = context(tbl, val).to_string();
        assert_eq!(exp, out);

        let exp = String::from("(() (()) ((((() ())))))");
        let val = parser::parse(reg, tbl, &exp).unwrap();
        let out = context(tbl, val).to_string();
        assert_eq!(exp, out);

        let exp = String::from("((1 2 3 4) ;Comment\n5)");
        let gnd = String::from("((1 2 3 4) 5)");
        let val = parser::parse(reg, tbl, &exp).unwrap();
        let out = context(tbl, val).to_string();
        assert_eq!(gnd, out);
    }
}
