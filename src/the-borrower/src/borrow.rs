use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Expr, ExprAssign, ExprGroup, FnArg, ItemFn, PatType, Token, Type, TypeReference, visit_mut::VisitMut};

struct RefBorrowAssigner<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for RefBorrowAssigner<'_> {
    fn visit_expr_assign_mut(&mut self, i: &mut ExprAssign) {
        let id = i.left.as_ref().into_token_stream().to_string();
        match self.make_mut.contains(&id) {
            false => (),
            true => {
                *i.left.as_mut() = syn::parse_quote!(format!("*{}", id));
            }
        }
    }

    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (),
            FnArg::Typed(t) => {
                let id = t.into_token_stream().to_string();
                match self.make_ref.contains(&id) {
                    true => {
                        t.ty = Box::from(Type::Reference(TypeReference {
                            and_token: Default::default(),
                            lifetime: None,
                            mutability: (Some(Default::default())),
                            elem: t.ty.clone(),
                        }))
                    },
                    false => {
                        match self.make_mut.contains(&id) {
                            false => (),
                            true => {

                            }
                        }
                    }
                }
            }
        }
    }
}

struct RefBorrower<'a> {
    fn_name: &'a str
}


impl VisitMut for RefBorrower<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            false => (),
            true => {
                let inputs = i.sig.inputs.iter().cloned();
                i.block.stmts.iter().for_each(|stmt|{
                    let check_assignment = |i: &ExprAssign| {
                        ()
                    };
                })
            }
        }
    }


}