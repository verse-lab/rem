use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, u64},
    sequence::delimited,
    IResult,
};

use crate::typ::RustType;
use crate::{labelling::Label, typ::CIntegralSize};

/// A combinator that takes a parser `inner` and produces a parser that also consumes both leading and
/// trailing whitespace, returning the output of `inner`.
pub fn ws<'a, F: 'a, O, E: nom::error::ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn rust_type(s: &str) -> IResult<&str, RustType> {
    fn int(s: &str) -> IResult<&str, RustType> {
        tag("c_int")(s).map(|(rest, _)| {
            (
                rest,
                RustType::CInt {
                    unsigned: false,
                    size: CIntegralSize::Int,
                },
            )
        })
    }
    fn ulong(s: &str) -> IResult<&str, RustType> {
        tag("c_ulong")(s).map(|(rest, _)| {
            (
                rest,
                RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Long,
                },
            )
        })
    }
    fn void(s: &str) -> IResult<&str, RustType> {
        tag("c_void")(s).map(|(rest, _)| (rest, RustType::CVoid))
    }
    fn i32_ty(s: &str) -> IResult<&str, RustType> {
        tag("i32")(s).map(|(rest, _)| (rest, RustType::I32))
    }
    fn isize(s: &str) -> IResult<&str, RustType> {
        tag("isize")(s).map(|(rest, _)| (rest, RustType::Isize))
    }
    fn usize(s: &str) -> IResult<&str, RustType> {
        tag("usize")(s).map(|(rest, _)| (rest, RustType::Usize))
    }
    fn size_t(s: &str) -> IResult<&str, RustType> {
        tag("size_t")(s).map(|(rest, _)| (rest, RustType::Usize))
    }
    fn uint(s: &str) -> IResult<&str, RustType> {
        tag("c_uint")(s).map(|(rest, _)| {
            (
                rest,
                RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Int,
                },
            )
        })
    }
    fn uchar(s: &str) -> IResult<&str, RustType> {
        tag("c_uchar")(s).map(|(rest, _)| {
            (
                rest,
                RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Char,
                },
            )
        })
    }
    fn int_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_c_int")(s).map(|(rest, _)| {
            (
                rest,
                RustType::Pointer(Box::new(RustType::CInt {
                    unsigned: false,
                    size: CIntegralSize::Int,
                })),
            )
        })
    }
    fn uint_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_c_uint")(s).map(|(rest, _)| {
            (
                rest,
                RustType::Pointer(Box::new(RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Int,
                })),
            )
        })
    }
    fn uchar_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_c_uchar")(s).map(|(rest, _)| {
            (
                rest,
                RustType::Pointer(Box::new(RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Char,
                })),
            )
        })
    }
    fn void_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_c_void")(s)
            .map(|(rest, _)| (rest, RustType::Pointer(Box::new(RustType::CVoid))))
    }
    // not sure how to make wildcard/recursively handle nested pointers so hardcoding it in for now...
    fn nested_int_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_mut_ptr_c_int")(s).map(|(rest, _)| {
            (
                rest,
                RustType::Pointer(Box::new(RustType::Pointer(Box::new(RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Int,
                })))),
            )
        })
    }
    fn nested_uint_ptr(s: &str) -> IResult<&str, RustType> {
        tag("mut_ptr_mut_ptr_c_uint")(s).map(|(rest, _)| {
            (
                rest,
                RustType::Pointer(Box::new(RustType::Pointer(Box::new(RustType::CInt {
                    unsigned: true,
                    size: CIntegralSize::Int,
                })))),
            )
        })
    }

    alt((
        nested_int_ptr,
        nested_uint_ptr,
        uint_ptr,
        uchar_ptr,
        void_ptr,
        int_ptr,
        uint,
        size_t,
        uchar,
        int,
        ulong,
        void,
        i32_ty,
        isize,
        usize,
    ))(s)
}

pub fn label(s: &str) -> IResult<&str, Label> {
    let (s, _) = tag("A")(s)?;
    let (s, digits) = u64(s)?;
    Ok((s, Label::of_raw(digits as usize)))
}
