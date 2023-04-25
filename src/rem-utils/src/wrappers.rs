use quote::ToTokens;

use crate::{pprint_ast, typ::RustType, CHRusty_build, CHRusty_parse};

/// Represents a wrapper struct that encodes the fact that a given pointer should implement indexed
#[derive(Clone, Debug)]
pub struct IndexWrapper {
    /// Depth of indexing supported by the wrapper
    /// i.e 1 => a[x]
    ///     2 => a[x][y]
    indirection: usize,
    /// base expression being wrapped
    expr: syn::Expr,
    /// type of inner expression
    ty: RustType,
}

impl std::fmt::Display for IndexWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexWrapper {{ depth: {}, expr: {} }}",
            self.indirection,
            self.expr.clone().into_token_stream().to_string()
        )
    }
}

fn extract_path_argument(argument: syn::PathArguments) -> Option<RustType> {
    match argument {
        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            args, ..
        }) if args.len() == 1 => match &args[0] {
            syn::GenericArgument::Type(ty) => Some(ty.clone().into()),
            _ => None,
        },
        _ => None,
    }
}

fn path_segments_to_str(path: syn::Path) -> Vec<(String, Option<RustType>)> {
    path.segments
        .into_iter()
        .map(|v| {
            let ident = v.ident.to_string();
            let typ = extract_path_argument(v.arguments);
            (ident, typ)
        })
        .collect()
}

fn is_tuple_struct(expr: &syn::ExprCall) -> bool {
    expr.args.len() == 1
}

fn unwrap_tuple_struct(expr: syn::Expr) -> (syn::Path, syn::Expr) {
    if let syn::Expr::Call(syn::ExprCall {
        func: box syn::Expr::Path(syn::ExprPath { path, .. }),
        args,
        ..
    }) = expr
    {
        assert!(args.len() == 1);
        (path, args[0].clone())
    } else {
        unreachable!()
    }
}

fn extract_tuple_struct(expr: &syn::Expr) -> (&syn::Path, &syn::Expr) {
    if let syn::Expr::Call(syn::ExprCall {
        func: box syn::Expr::Path(syn::ExprPath { path, .. }),
        args,
        ..
    }) = expr
    {
        assert!(args.len() == 1);
        (path, &args[0])
    } else {
        unreachable!()
    }
}

fn is_tuple_call(expr: &syn::Expr) -> bool {
    if let syn::Expr::Call(cs) = expr {
        is_tuple_struct(cs)
    } else {
        false
    }
}

impl IndexWrapper {
    pub fn new(indirection: usize, expr: syn::Expr, ty: RustType) -> Self {
        IndexWrapper {
            indirection,
            expr,
            ty,
        }
    }

    pub fn indirection(&self) -> usize {
        self.indirection
    }

    pub fn base_expr(&self) -> &syn::Expr {
        &self.expr
    }

    pub fn base_ty(&self) -> &RustType {
        &self.ty
    }

    /// Test whether an expression is indeed an index wrapper
    pub fn is_index_wrapper(expr: &syn::Expr) -> bool {
        match &expr {
            syn::Expr::Call(expr_call) if is_tuple_struct(expr_call) => {
                let (path, _) = extract_tuple_struct(expr);
                let path_segments = path_segments_to_str(path.clone());
                let elts = path_segments
                    .iter()
                    .map(|(v, _)| v.as_str())
                    .collect::<Vec<_>>();
                &elts[..] == &["chrusty", "IndexWrapperFinal"]
            }
            _ => false,
        }
    }

    /// Folds over the calls in a wrapper in order from IndexWrapperFinal to IndexWrapperBase
    pub fn fold_calls<P, O>(mut f: P, expr: &syn::Expr) -> Vec<O>
    where
        P: FnMut(&syn::Expr) -> O,
    {
        let mut acc = vec![];
        let mut expr = expr;
        while is_tuple_call(expr) {
            let (path, next_expr) = extract_tuple_struct(expr);
            let slice = path_segments_to_str(path.clone());
            let elts = slice
                .iter()
                .map(|(v, ty)| (v.as_str(), ty))
                .collect::<Vec<_>>();
            match &elts[..] {
                [("chrusty", _), ("IndexWrapperFinal", _)] => {
                    acc.push(f(expr));
                    expr = next_expr
                }
                [("chrusty", _), ("IndexWrapper", _)] => {
                    acc.push(f(expr));
                    expr = next_expr
                }
                [("chrusty", _), ("IndexWrapperBase", _)] => {
                    acc.push(f(expr));
                    break;
                }
                elts => panic!("unexpected index wrapper structure {:?}", elts),
            }
        }
        acc
    }
}

impl Into<syn::Expr> for IndexWrapper {
    fn into(self) -> syn::Expr {
        fn wrap_with_constructor(name: &str, expr: syn::Expr, typ: Option<syn::Type>) -> syn::Expr {
            let last_segment = {
                let mut base = CHRusty_parse!((name) as syn::PathSegment);
                if let Some(typ) = typ {
                    base.arguments = syn::PathArguments::AngleBracketed(
                        CHRusty_build!(syn::AngleBracketedGenericArguments {
                            args: [syn::GenericArgument::Type(typ)].into_iter().collect(),
                            // Note: explicitly setting the colon2
                            // token to some is important, else the
                            // code will not be valid rust
                            colon2_token: Some(Default::default());
                            default![lt_token, gt_token]
                        }),
                    )
                }
                base
            };

            syn::Expr::Call(CHRusty_build!(syn::ExprCall {
                func: box syn::Expr::Path(CHRusty_build!(syn::ExprPath{
                    path: syn::Path {
                        leading_colon: None,
                        segments: [
                            CHRusty_parse!("chrusty" as syn::PathSegment),
                            last_segment
                        ].into_iter().collect()
                    };
                    default![attrs, qself]
                })),
                args: [expr].into_iter().collect();
                default![attrs,paren_token]
            }))
        }
        let mut expr: syn::Expr =
            wrap_with_constructor("IndexWrapperBase", self.expr, Some(self.ty.into()));

        for _ in 1..self.indirection {
            expr = wrap_with_constructor("IndexWrapper", expr, None)
        }

        wrap_with_constructor("IndexWrapperFinal", expr, None)
    }
}

impl From<syn::Expr> for IndexWrapper {
    fn from(expr: syn::Expr) -> Self {
        let mut expr = match &expr {
            syn::Expr::Call(expr_call) if is_tuple_struct(expr_call) => {
                let (path, inner_expr) = unwrap_tuple_struct(expr);
                let path_segments = path_segments_to_str(path);
                let elts = path_segments
                    .iter()
                    .map(|(v, _)| v.as_str())
                    .collect::<Vec<_>>();
                assert!(&elts[..] == &["chrusty", "IndexWrapperFinal"]);
                inner_expr
            }
            expr => panic!(
                "unexpected structure for RustIndexWrapper {:?}",
                pprint_ast!(expr)
            ),
        };

        let base_expr;
        let base_typ;
        let mut indirection = 1;

        loop {
            let (path, inner_expr) = unwrap_tuple_struct(expr);
            let slice = path_segments_to_str(path.clone());
            let elts = slice
                .iter()
                .map(|(v, ty)| (v.as_str(), ty))
                .collect::<Vec<_>>();
            match &elts[..] {
                [("chrusty", _), ("IndexWrapper", _)] => {
                    indirection += 1;
                    expr = inner_expr
                }
                [("chrusty", _), ("IndexWrapperBase", Some(ty))] => {
                    base_expr = inner_expr;
                    base_typ = ty.clone();
                    break;
                }
                elts => panic!("unexpected index wrapper structure {:?}", elts),
            }
        }
        let expr = base_expr;
        let ty = base_typ;

        IndexWrapper {
            indirection,
            expr,
            ty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_wrapper_conversion_works_for_1() {
        let base_expr = CHRusty_parse!("x.as_mut_ptr()" as syn::Expr);
        let base_ty = CHRusty_parse!("*mut i32" as syn::Type).into();

        let wrapper: IndexWrapper = IndexWrapper::new(1, base_expr, base_ty);

        let wrapper_expr: syn::Expr = wrapper.into();

        let wrapper: IndexWrapper = wrapper_expr.into();

        assert_eq!(wrapper.indirection, 1);
        let base_ty: syn::Type = wrapper.ty.into();
        assert_eq!(&pprint_ast!(base_ty), "* mut i32")
    }

    #[test]
    fn test_index_wrapper_conversion_works_for_2() {
        let base_expr = CHRusty_parse!("x.as_mut_ptr()" as syn::Expr);
        let base_ty = CHRusty_parse!("*mut *mut i32" as syn::Type).into();

        let wrapper: IndexWrapper = IndexWrapper::new(2, base_expr, base_ty);

        let wrapper_expr: syn::Expr = wrapper.into();

        let wrapper: IndexWrapper = wrapper_expr.into();

        assert_eq!(wrapper.indirection, 2);
        let base_ty: syn::Type = wrapper.ty.into();
        assert_eq!(&pprint_ast!(base_ty), "* mut * mut i32")
    }

    #[test]
    fn test_index_wrapper_conversion_works_for_3() {
        let base_expr = CHRusty_parse!("x.as_mut_ptr()" as syn::Expr);
        let base_ty = CHRusty_parse!("* mut * mut * mut i32" as syn::Type).into();

        let wrapper: IndexWrapper = IndexWrapper::new(3, base_expr, base_ty);

        let wrapper_expr: syn::Expr = wrapper.into();

        let wrapper: IndexWrapper = wrapper_expr.into();

        assert_eq!(wrapper.indirection, 3);
        let base_ty: syn::Type = wrapper.ty.into();
        assert_eq!(&pprint_ast!(base_ty), "* mut * mut * mut i32")
    }

    #[test]
    fn test_index_wrapper_has_correct_internal_structure() {
        let base_expr = CHRusty_parse!("x.as_mut_ptr()" as syn::Expr);
        let base_ty = CHRusty_parse!("* mut * mut * mut i32" as syn::Type).into();

        let wrapper: IndexWrapper = IndexWrapper::new(3, base_expr, base_ty);

        let wrapper_expr: syn::Expr = wrapper.into();
        assert_eq!(&pprint_ast!(wrapper_expr), "chrusty :: IndexWrapperFinal (chrusty :: IndexWrapper (chrusty :: IndexWrapper (chrusty :: IndexWrapperBase :: < * mut * mut * mut i32 > (x . as_mut_ptr ()))))");
    }
}
