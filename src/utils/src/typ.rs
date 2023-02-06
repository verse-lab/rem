use ena::unify::UnifyValue;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use syn::punctuated::Punctuated;
use syn::{FieldsNamed, Path, PathSegment, Type, TypeArray};

/// Mapping of function names to type signatures
pub type TypeMap = HashMap<crate::location::Loc, RustTypeSignature>;

#[derive(Debug)]
pub enum Error {
    UnUnifiableTypes(RustType, RustType),
}

pub type ProgramTypeContext = (
    HashMap<syn::Ident, RustType>,
    HashMap<syn::Ident, RustStruct>,
);

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct TVar(pub usize);

impl From<&str> for TVar {
    fn from(s: &str) -> Self {
        if !s.starts_with("T") {
            panic!("invalid assumption: unknown generic var \"{}\"", s)
        }
        let ind = s
            .trim_start_matches('T')
            .parse()
            .expect("Index for generic could not be extracted");
        TVar(ind)
    }
}

impl From<String> for TVar {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}

impl std::fmt::Display for TVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RustMutability {
    Immutable,
    Mutable,
}

impl std::fmt::Display for RustMutability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustMutability::Immutable => write!(f, "immutable"),
            RustMutability::Mutable => write!(f, "mutable"),
        }
    }
}

impl From<Option<syn::token::Mut>> for RustMutability {
    fn from(mut_: Option<syn::token::Mut>) -> Self {
        match mut_ {
            Some(_) => RustMutability::Mutable,
            None => RustMutability::Immutable,
        }
    }
}

impl Into<Option<syn::token::Mut>> for RustMutability {
    fn into(self) -> Option<syn::token::Mut> {
        match self {
            RustMutability::Mutable => Some(Default::default()),
            RustMutability::Immutable => None,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CIntegralSize {
    Char,
    Short,
    Int,
    Long,
    LongLong,
}

impl std::fmt::Display for CIntegralSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CIntegralSize::Char => write!(f, "char"),
            CIntegralSize::Short => write!(f, "short"),
            CIntegralSize::Int => write!(f, "int"),
            CIntegralSize::Long => write!(f, "long"),
            CIntegralSize::LongLong => write!(f, "longlong"),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum CFloatSize {
    Float,
    Double,
}

impl std::fmt::Display for CFloatSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CFloatSize::Float => write!(f, "float"),
            CFloatSize::Double => write!(f, "double"),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum RustType {
    CVoid,
    CInt {
        unsigned: bool,
        size: CIntegralSize,
    },
    CFloat(CFloatSize),
    CAlias(syn::Ident),

    Array(Box<RustType>, usize),

    Option(Box<RustType>),
    Vec(Box<RustType>),
    Unit,
    I32,
    U8,
    SizeT,
    Isize,
    Usize,
    TVar(TVar), //rust types

    Never,

    ExternFn(Vec<Box<RustType>>, bool, Box<RustType>),

    /// immutable reference
    Reference(RustMutability, Box<RustType>),
    /// *mut T
    Pointer(Box<RustType>),
}

impl RustType {
    fn uses(&self, set: &mut HashSet<syn::Ident>) {
        match self {
            RustType::CAlias(id) => {
                set.insert(id.clone());
                ()
            }
            RustType::Option(ty)
            | RustType::Vec(ty)
            | RustType::Reference(_, ty)
            | RustType::Pointer(ty)
            | RustType::Array(ty, _) => ty.uses(set),

            // RustType::ExternFn(args, _, out_ty) => {
            //     for arg in args.iter() {
            //         arg.uses(set)
            //     }
            //     out_ty.uses(set)
            // }
            _ => (),
        }
    }

    fn resolve_checked(
        &mut self,
        path: &mut HashSet<syn::Ident>,
        ctxt: &ProgramTypeContext,
    ) -> bool {
        match self {
            // recursion check
            RustType::CAlias(id) if path.contains(id) => true,
            RustType::CAlias(id) if !path.contains(id) => {
                // add the visited alias to the path
                path.insert(id.clone());
                // if type alias to a defined struct, then we good boys
                if ctxt.1.contains_key(id) {
                    false
                } else {
                    match ctxt.0.get(id) {
                        Some(inner) => {
                            // update self to be the inner type
                            *self = inner.clone();
                            // recursive
                            self.resolve_checked(path, ctxt)
                        }
                        None => {
                            log::warn!("attempted to resolve type {} that has no defined alias or definition", id.to_string());
                            false
                        }
                    }
                }
            }
            RustType::Option(elt)
            | RustType::Vec(elt)
            | RustType::Pointer(elt)
            | RustType::Reference(_, elt)
            | RustType::Array(elt, _) => elt.resolve_checked(path, ctxt),
            RustType::ExternFn(args, _, out) => {
                let mut any_rec = false;
                let mut base_path = path.clone();
                for arg in args.iter_mut() {
                    let mut rec_path = base_path.clone();
                    any_rec |= arg.resolve_checked(&mut rec_path, ctxt);
                    path.extend(rec_path.into_iter());
                }
                any_rec |= out.resolve_checked(&mut base_path, ctxt);
                path.extend(base_path.into_iter());
                any_rec
            }
            _ => false,
        }
    }

    /// Resolves a type according to the type context, avoiding loops, returning the list of types visited
    pub fn resolve(&mut self, ctxt: &ProgramTypeContext) -> HashSet<syn::Ident> {
        let mut set = HashSet::new();
        self.resolve_checked(&mut set, ctxt);
        set
    }
}

impl std::fmt::Display for RustType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustType::Never => write!(f, "never"),
            RustType::Array(box ty, size) => write!(f, "array({}, {})", ty, size),
            RustType::Option(box ty) => write!(f, "option({})", ty),
            RustType::Vec(box ty) => write!(f, "vec({})", ty),
            RustType::CInt { unsigned, size } => {
                write!(f, "c_{}{}", if *unsigned { "u" } else { "" }, size)
            }
            RustType::CFloat(size) => write!(f, "c_{}", size),
            RustType::CVoid => write!(f, "c_void"),
            RustType::CAlias(ident) => write!(f, "{}", ident),
            RustType::Unit => write!(f, "()"),
            RustType::SizeT => write!(f, "size_t"),
            RustType::U8 => write!(f, "u8"),
            RustType::I32 => write!(f, "i32"),
            RustType::Isize => write!(f, "isize"),
            RustType::Usize => write!(f, "usize"),
            RustType::TVar(tvar) => write!(f, "{}", tvar),
            RustType::Pointer(box x) => write!(f, "mut_ptr_{}", x),
            RustType::Reference(mt, box x) => write!(f, "ref_{}_{}", mt, x),
            RustType::ExternFn(args, variadic, body) => write!(
                f,
                "extern_fn_({}, {}, {})",
                variadic,
                args.iter()
                    .map(|v| format!("{}", v))
                    .collect::<Vec<_>>()
                    .join(","),
                body
            ),
        }
    }
}

impl Into<Type> for RustType {
    fn into(self) -> Type {
        match self {
            RustType::Never => syn::Type::Never(syn::TypeNever {
                bang_token: Default::default(),
            }),
            RustType::Array(box ty, size) => Type::Array(syn::TypeArray {
                bracket_token: Default::default(),
                elem: Box::new(ty.into()),
                semi_token: Default::default(),
                len: syn::parse_str::<syn::Expr>(&format!("{}", size)).unwrap(),
            }),
            RustType::Option(box ty) => Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: [PathSegment {
                        ident: syn::parse_str::<syn::Ident>("Option").unwrap(),
                        arguments: syn::PathArguments::AngleBracketed(
                            syn::AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Default::default(),
                                args: [syn::GenericArgument::Type(ty.into())]
                                    .into_iter()
                                    .collect(),
                                gt_token: Default::default(),
                            },
                        ),
                    }]
                    .into_iter()
                    .collect(),
                },
            }),
            RustType::Vec(box ty) => Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: [PathSegment {
                        ident: syn::parse_str::<syn::Ident>("Vec").unwrap(),
                        arguments: syn::PathArguments::AngleBracketed(
                            syn::AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Default::default(),
                                args: [syn::GenericArgument::Type(ty.into())]
                                    .into_iter()
                                    .collect(),
                                gt_token: Default::default(),
                            },
                        ),
                    }]
                    .into_iter()
                    .collect(),
                },
            }),
            RustType::Unit => syn::parse_str::<Type>("()").unwrap(),
            ty @ RustType::CInt { .. } => syn::parse_str::<Type>(&format!("libc::{}", ty)).unwrap(),
            ty @ RustType::CFloat(_) => syn::parse_str::<Type>(&format!("libc::{}", ty)).unwrap(),
            RustType::CVoid => syn::parse_str::<Type>("libc::c_void").unwrap(),

            RustType::CAlias(ident) => Type::Path(syn::TypePath {
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: [syn::PathSegment::from(ident)].into_iter().collect(),
                },
            }),
            RustType::SizeT => syn::parse_str::<Type>("size_t").unwrap(),
            RustType::U8 => syn::parse_str::<Type>("u8").unwrap(),
            RustType::I32 => syn::parse_str::<Type>("i32").unwrap(),
            RustType::Isize => syn::parse_str::<Type>("isize").unwrap(),
            RustType::Usize => syn::parse_str::<Type>("usize").unwrap(),
            RustType::TVar(n) => syn::parse_str::<Type>(&format!("{}", n)).unwrap(),
            RustType::Pointer(box v) => Type::Ptr(syn::TypePtr {
                const_token: None,
                mutability: Some(Default::default()),
                elem: Box::new(v.into()),
                star_token: Default::default(),
            }),
            RustType::Reference(muta, box v) => Type::Reference(syn::TypeReference {
                and_token: Default::default(),
                mutability: muta.into(),
                elem: Box::new(v.into()),
                lifetime: None,
            }),
            RustType::ExternFn(args, variadic, box res) => {
                let inputs = if variadic {
                    args.into_iter()
                        .map(|f| syn::BareFnArg {
                            attrs: Default::default(),
                            name: None,
                            ty: (*f).into(),
                        })
                        .collect()
                } else {
                    args.into_iter()
                        .map(|f| syn::BareFnArg {
                            attrs: Default::default(),
                            name: None,
                            ty: (*f).into(),
                        })
                        .chain(
                            [syn::BareFnArg {
                                attrs: Default::default(),
                                name: None,
                                ty: Type::Verbatim("...".to_token_stream()),
                            }]
                            .into_iter(),
                        )
                        .collect()
                };

                Type::BareFn(syn::TypeBareFn {
                    lifetimes: None,
                    unsafety: Some(Default::default()),
                    abi: Some(syn::Abi {
                        extern_token: Default::default(),
                        name: Some(syn::parse_str::<syn::LitStr>("\"C\"").unwrap()),
                    }),
                    fn_token: Default::default(),
                    paren_token: Default::default(),
                    inputs,
                    variadic: None,
                    output: syn::ReturnType::Type(Default::default(), Box::new(res.into())),
                })
            }
        }
    }
}

impl From<Type> for RustType {
    fn from(ty: Type) -> Self {
        match ty {
            Type::Path(syn::TypePath {
                path: Path { segments, .. },
                ..
            }) if segments.last().is_some_and(|segment| {
                segment.ident == "Option" && !segment.arguments.is_empty()
            }) =>
            {
                let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) =
                        &segments.last().unwrap().arguments else {
                        panic!("found use of unsupported syntactic construct {} in code", segments.to_token_stream().to_string())
                    };

                let syn::GenericArgument::Type(ty) = &args[0] else {
                        panic!("found use of unsupported syntactic construct {} in code", segments.to_token_stream().to_string())
                    };

                RustType::Option(Box::new(ty.clone().into()))
            }
            Type::Path(syn::TypePath {
                path: Path { segments, .. },
                ..
            }) if segments
                .last()
                .is_some_and(|segment| segment.ident == "Vec" && !segment.arguments.is_empty()) =>
            {
                let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) =
                        &segments.last().unwrap().arguments else {
                        panic!("found use of unsupported syntactic construct {} in code", segments.to_token_stream().to_string())
                    };

                let syn::GenericArgument::Type(ty) = &args[0] else {
                        panic!("found use of unsupported syntactic construct {} in code", segments.to_token_stream().to_string())
                    };

                RustType::Vec(Box::new(ty.clone().into()))
            }
            Type::Path(syn::TypePath {
                path: ref path @ Path { ref segments, .. },
                ..
            }) if segments
                .last()
                .is_some_and(|segment| segment.arguments.is_empty()) =>
            {
                let ident = &segments.last().unwrap().ident;

                if !(segments.len() == 1)
                    && !(segments.len() > 1 && segments[0].ident == "libc")
                    && !(segments.len() > 3
                        && segments[0].ident == "std"
                        && segments[1].ident == "os"
                        && segments[2].ident == "raw")
                {
                    log::warn!("found use of non libc type {:?}, optimistically assuming nothing crazy is going on",
                               path.to_token_stream().to_string()
                    )
                }

                use RustType::*;

                match ident.to_string().as_str() {
                    "isize" => Isize,
                    "i32" => I32,
                    "size_t" => SizeT,
                    "u8" => U8,
                    "usize" => Usize,
                    "c_float" => CFloat(CFloatSize::Float),
                    "c_double" => CFloat(CFloatSize::Double),

                    "c_char" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Char,
                    },
                    "c_schar" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Char,
                    },
                    "c_uchar" => CInt {
                        unsigned: true,
                        size: CIntegralSize::Char,
                    },
                    "c_short" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Short,
                    },
                    "c_ushort" => CInt {
                        unsigned: true,
                        size: CIntegralSize::Short,
                    },

                    "c_int" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Int,
                    },
                    "c_uint" => CInt {
                        unsigned: true,
                        size: CIntegralSize::Int,
                    },

                    "c_long" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Long,
                    },
                    "c_ulong" => CInt {
                        unsigned: true,
                        size: CIntegralSize::Long,
                    },

                    "c_longlong" => CInt {
                        unsigned: false,
                        size: CIntegralSize::Long,
                    },
                    "c_ulonglong" => CInt {
                        unsigned: true,
                        size: CIntegralSize::Long,
                    },

                    "c_void" => CVoid,

                    _txt => RustType::CAlias(ident.clone()),
                }
            }
            Type::Ptr(syn::TypePtr {
                const_token,
                mutability,
                elem: box ty,
                ..
            }) => {
                match (const_token, mutability) {
                    (None, Some(_)) => RustType::Pointer(Box::new(ty.into())), //case where we have *mut T
                    (Some(_), None) => RustType::Pointer(Box::new(ty.into())), //case where we have *const T
                    (_, _) => {
                        todo!()
                    }
                }
            }
            Type::Reference(syn::TypeReference {
                lifetime: None,
                mutability,
                elem: box elem,
                ..
            }) => RustType::Reference(mutability.into(), Box::new(elem.into())),

            Type::Tuple(syn::TypeTuple { elems, .. }) if elems.len() == 0 => RustType::Unit,

            Type::BareFn(syn::TypeBareFn {
                unsafety: Some(_),
                abi: Some(_),
                inputs,
                output,
                variadic: None,
                ..
            }) => {
                let mut variadic = false;
                let inputs = inputs
                    .into_iter()
                    .map(|v| match v.ty {
                        Type::Verbatim(_v) => {
                            variadic = true;
                            None
                        }
                        _ => Some(Box::new(v.ty.into())),
                    })
                    .flatten()
                    .collect();

                let output = match output {
                    syn::ReturnType::Default => RustType::Unit,
                    syn::ReturnType::Type(_, box ty) => ty.into(),
                };
                RustType::ExternFn(inputs, variadic, Box::new(output))
            }

            Type::Array(TypeArray {
                elem: box ty,
                len:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Int(i),
                        ..
                    }),
                ..
            }) => RustType::Array(Box::new(ty.into()), i.base10_parse().unwrap()),
            Type::Never(_) => RustType::Never,
            _ => panic!("unsupported type {:?}", ty.to_token_stream().to_string()),
        }
    }
}

#[derive(Clone, Debug, Hash)]
pub struct RustStruct {
    name: syn::Ident,
    fields: Vec<(syn::Ident, RustType)>,
}

impl std::fmt::Display for RustStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "struct {} {{ {} }}",
            self.name,
            self.fields
                .iter()
                .map(|(name, ty)| format!("{}: {}", name, ty))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

impl RustStruct {
    pub fn name(&self) -> &syn::Ident {
        &self.name
    }

    pub fn fields(&self) -> &Vec<(syn::Ident, RustType)> {
        &self.fields
    }

    /// Returns a list of all the structs that this struct references
    pub fn uses(&self) -> HashSet<syn::Ident> {
        let mut uses = HashSet::new();
        for (_, ty) in self.fields.iter() {
            ty.uses(&mut uses);
        }
        uses
    }

    /// Resolves a structs types, and returns the list of type names it references
    pub fn resolve(&mut self, ctxt: &ProgramTypeContext) -> HashSet<syn::Ident> {
        let mut acc = HashSet::new();
        for (_, ty) in self.fields.iter_mut() {
            acc.extend(ty.resolve(ctxt).into_iter())
        }
        acc
    }
}

impl From<syn::ItemStruct> for RustStruct {
    fn from(i: syn::ItemStruct) -> Self {
        if i.attrs.len() == 0 {
            log::warn!("skipping unknown attributes {:?}", i.attrs)
        }
        if i.generics.params.len() > 0 {
            panic!("found unsupported use of generic struct {:?} - @Bryan, if you are implementing lifetimes or something, talk to me first.",
            i.generics)
        }

        let syn::Fields::Named(FieldsNamed { named, .. }) = i.fields else {
            panic!("found unsupported struct declaration {}", i.to_token_stream().to_string())
        };
        let name = i.ident;
        let fields = named
            .into_iter()
            .map(|v| (v.ident.unwrap(), v.ty.into()))
            .collect();
        RustStruct { name, fields }
    }
}

impl RustStruct {}

#[derive(Clone, Debug, Hash)]
pub enum RustTypeConstraint {
    /// Index(T1, T2) represents Index<T1, Output=T2>
    Index(RustType, RustType),
    /// IndexMut(T1, T2) represents IndexMut<T1, Output=T2>
    IndexMut(RustType, RustType),
}

impl Into<syn::TypeParamBound> for RustTypeConstraint {
    fn into(self) -> syn::TypeParamBound {
        match self {
            // Index<T1, Output=T2>
            RustTypeConstraint::Index(t1, t2) => {
                let mut path = syn::Path {
                    leading_colon: None,
                    segments: Punctuated::new(),
                };
                let mut args = Punctuated::new();
                args.push(syn::GenericArgument::Type(t1.into()));
                args.push(syn::GenericArgument::Binding(syn::Binding {
                    ident: syn::parse_str::<syn::Ident>("Output").unwrap(),
                    eq_token: Default::default(),
                    ty: t2.into(),
                }));

                path.segments.push(syn::PathSegment {
                    ident: syn::parse_str::<syn::Ident>("Index").unwrap(),
                    arguments: syn::PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Default::default(),
                            args,
                            gt_token: Default::default(),
                        },
                    ),
                });

                syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path,
                })
            }
            RustTypeConstraint::IndexMut(t1, t2) => {
                let mut path = syn::Path {
                    leading_colon: None,
                    segments: Punctuated::new(),
                };
                let mut args = Punctuated::new();
                args.push(syn::GenericArgument::Type(t1.into()));
                args.push(syn::GenericArgument::Binding(syn::Binding {
                    ident: syn::parse_str::<syn::Ident>("Output").unwrap(),
                    eq_token: Default::default(),
                    ty: t2.into(),
                }));

                path.segments.push(syn::PathSegment {
                    ident: syn::parse_str::<syn::Ident>("IndexMut").unwrap(),
                    arguments: syn::PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Default::default(),
                            args,
                            gt_token: Default::default(),
                        },
                    ),
                });

                syn::TypeParamBound::Trait(syn::TraitBound {
                    paren_token: None,
                    modifier: syn::TraitBoundModifier::None,
                    lifetimes: None,
                    path,
                })
            }
        }
    }
}

impl From<syn::TypeParamBound> for RustTypeConstraint {
    fn from(ty: syn::TypeParamBound) -> Self {
        match ty {
            syn::TypeParamBound::Trait(syn::TraitBound {
                path: syn::Path { segments, .. },
                ..
            }) if segments.len() == 1 => {
                let segment = segments[0].clone();
                let trait_name = segment.ident.to_string();
                match (trait_name.as_str(), segment.arguments) {
                    ("Index", syn::PathArguments::AngleBracketed(args)) => {
                        let in_ty = match args.args[0].clone() {
                            syn::GenericArgument::Type(ty) => ty,
                            _ => panic!("invalid constraint structure {:#?}", segments),
                        };
                        let out_ty = match args.args[1].clone() {
                            syn::GenericArgument::Binding(binding) => binding.ty,
                            _ => panic!("invalid constraint structure {:#?}", segments),
                        };
                        RustTypeConstraint::Index(in_ty.into(), out_ty.into())
                    }
                    ("IndexMut", syn::PathArguments::AngleBracketed(args)) => {
                        let in_ty = match args.args[0].clone() {
                            syn::GenericArgument::Type(ty) => ty,
                            _ => panic!("invalid constraint structure {:#?}", segments),
                        };
                        let out_ty = match args.args[1].clone() {
                            syn::GenericArgument::Binding(binding) => binding.ty,
                            _ => panic!("invalid constraint structure {:#?}", segments),
                        };
                        RustTypeConstraint::IndexMut(in_ty.into(), out_ty.into())
                    }
                    _ => panic!("unsupported type constraint {:#?}", segments),
                }
            }
            ty => panic!("unsupported type constraint {:#?}", ty),
        }
    }
}

impl std::fmt::Display for RustTypeConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RustTypeConstraint::Index(ind_ty, out_ty) => write!(f, "Index<{},{}>", ind_ty, out_ty),
            RustTypeConstraint::IndexMut(ind_ty, out_ty) => {
                write!(f, "IndexMut<{},{}>", ind_ty, out_ty)
            }
        }
    }
}

#[derive(Clone)]
pub struct RustTypeSignature {
    name: String,
    constraints: Vec<(TVar, Vec<RustTypeConstraint>)>,
    args: Vec<(String, RustType)>,
    out_ty: Option<RustType>,
}

impl RustTypeSignature {
    pub fn constraints(&self) -> &Vec<(TVar, Vec<RustTypeConstraint>)> {
        &self.constraints
    }

    pub fn args(&self) -> &Vec<(String, RustType)> {
        &self.args
    }
}

impl From<syn::Signature> for RustTypeSignature {
    fn from(sig: syn::Signature) -> Self {
        let name = sig.ident.to_string();
        let constraints = sig
            .generics
            .type_params()
            .into_iter()
            .map(|param| {
                let tvar = param.ident.to_string().into();
                let bounds = param
                    .bounds
                    .clone()
                    .into_pairs()
                    .map(|v| v.into_value())
                    .map(|v| v.into())
                    .collect();
                (tvar, bounds)
            })
            .collect::<Vec<_>>();
        let args = sig
            .inputs
            .into_pairs()
            .map(|v| match v.into_value() {
                syn::FnArg::Typed(syn::PatType {
                    pat: box syn::Pat::Ident(syn::PatIdent { ident, .. }),
                    ty: box ty,
                    ..
                }) => (ident.to_string(), ty.into()),
                v => panic!("unsupported pattern {:?} in signature", v),
            })
            .collect();
        let out_ty = match sig.output {
            syn::ReturnType::Default => None,
            syn::ReturnType::Type(_, box ty) => Some(ty.into()),
        };
        RustTypeSignature {
            name,
            constraints,
            args,
            out_ty,
        }
    }
}

impl std::fmt::Display for RustTypeSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}<", self.name)?;

        {
            let constraints = self.constraints.iter().map(|v| Some(v)).intersperse(None);
            for constraint in constraints {
                match constraint {
                    None => write!(f, ", ")?,
                    Some((tvar, constraints)) => {
                        write!(f, "{}: ", tvar)?;
                        let constraints = constraints
                            .iter()
                            .map(|v| format!("{}", v))
                            .intersperse(" + ".into());
                        for opt in constraints {
                            write!(f, "{}", opt)?
                        }
                    }
                }
            }
        }
        write!(f, ">(")?;
        {
            let mut args = self.args.iter();
            let mut next = args.next();
            while next.is_some() {
                let (name, ty) = next.unwrap();
                write!(f, "{}: {}", name, ty)?;
                next = args.next();
                if next.is_some() {
                    write!(f, ",")?;
                }
            }
        }
        write!(f, ")")?;
        match &self.out_ty {
            None => (),
            Some(ty) => writeln!(f, " -> {}", ty)?,
        }

        Ok(())
    }
}

#[derive(Debug, Default, Clone)]
pub struct CTypeContextCollector {
    aliases: HashMap<syn::Ident, RustType>,
    structs: HashMap<syn::Ident, RustStruct>,
}

impl CTypeContextCollector {
    pub fn to_type_context(self) -> ProgramTypeContext {
        (self.aliases, self.structs)
    }
}

impl<'ast> syn::visit::Visit<'ast> for CTypeContextCollector {
    fn visit_item_type(&mut self, i: &'ast syn::ItemType) {
        let typ: RustType = (&*i.ty).clone().into();

        self.aliases.insert(i.ident.clone(), typ);
    }

    fn visit_item_struct(&mut self, i: &'ast syn::ItemStruct) {
        self.structs.insert(i.ident.clone(), i.clone().into());
    }

    fn visit_item_enum(&mut self, i: &'ast syn::ItemEnum) {
        panic!(
            "found use of unsupported enum {:?} declaration",
            i.to_token_stream().to_string()
        )
    }
}

fn check_recursive(
    checking: &syn::Ident,
    current: syn::Ident,
    mut path: HashSet<syn::Ident>,
    usage_map: &HashMap<syn::Ident, HashSet<syn::Ident>>,
) -> bool {
    // hit a recursion, exit
    if path.contains(&current) {
        false
    } else {
        match usage_map.get(&current) {
            None => false,
            Some(usages) => {
                path.insert(current);
                if usages.contains(checking) {
                    true
                } else {
                    usages
                        .iter()
                        .any(|id| check_recursive(checking, id.clone(), path.clone(), usage_map))
                }
            }
        }
    }
}

pub fn normalize_type_context(ctxt: &mut ProgramTypeContext) -> HashSet<syn::Ident> {
    let mut usage_map = HashMap::new();
    let ref_ctx = (ctxt.0.clone(), ctxt.1.clone());
    for (_name, st) in ctxt.0.iter_mut() {
        st.resolve(&ref_ctx);
    }
    for (name, st) in ctxt.1.iter_mut() {
        st.resolve(&ref_ctx);
        usage_map.insert(name.clone(), st.uses());
    }

    let mut recursive = HashSet::new();

    for (name, uses) in usage_map.iter() {
        if uses.contains(name) || check_recursive(name, name.clone(), HashSet::new(), &usage_map) {
            recursive.insert(name.clone());
        }
    }
    for (id, alias) in ctxt.0.iter().clone() {
        let mut uses = HashSet::new();
        alias.uses(&mut uses);
        if recursive.intersection(&uses).any(|_| true) {
            recursive.insert(id.clone());
        }
    }
    recursive
}

impl UnifyValue for RustType {
    type Error = Error;

    fn unify_values(value1: &Self, value2: &Self) -> Result<Self, Self::Error> {
        match (value1, value2) {
            (t1, t2) if t1 == t2 => Ok(t1.clone()),
            (RustType::TVar(t1), RustType::TVar(t2)) if t1 == t2 => Ok(RustType::TVar(*t1)),
            (RustType::Pointer(box x), RustType::Pointer(box y)) => {
                let contents = Self::unify_values(x, y)?;
                Ok(RustType::Pointer(Box::new(contents)))
            }
            (t1, t2) => Err(Error::UnUnifiableTypes(t1.clone(), t2.clone())),
        }
    }
}
