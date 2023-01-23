use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Expr, ExprAssign, ExprCall, ExprGroup, FnArg, Item, ItemFn, PatType, Token, Type, TypeReference, visit_mut::VisitMut};
use syn::visit::Visit;

struct RefBorrowAssigner<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for RefBorrowAssigner<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let id = i.into_token_stream().to_string();
        match self.make_mut.contains(&id) || self.make_ref.contains(&id) {
            false => (),
            true => {
                *i = syn::parse_quote!(format!("*{}", id));
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
                            mutability: (Some(syn::parse_quote!("mut"))),
                            elem: t.ty.clone(),
                        }))
                    },
                    false => {
                        match self.make_mut.contains(&id) {
                            false => (),
                            true => {
                                t.ty = Box::from(Type::Reference(TypeReference {
                                    and_token: Default::default(),
                                    lifetime: None,
                                    mutability: None,
                                    elem: t.ty.clone(),
                                }))
                            }
                        }
                    }
                }
            }
        }
    }
}

struct CallerCheckCallee<'a> {
    callee_fn_name: &'a str,
    found: bool,
}

impl VisitMut for CallerCheckCallee<'_>{
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id= i.func.as_ref().into_token_stream().to_string();
        match id == self.callee_fn_name {
            false => (),
            true => self.found = true,
        }
    }
}

struct CallerCheckInput<'a> {
    input: &'a Vec<String>,
}

impl VisitMut for CallerCheckInput<'_> {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        let id =
    }
}

struct CallerHelper<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CallerHelper<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                let inputs = i.sig.inputs.iter().cloned();
                let inputs_str : Vec<String> = inputs.map(|fn_arg| fn_arg.into_token_stream().to_string()).collect();
                let mut check_callee = CallerCheckCallee{ callee_fn_name: self.callee_fn_name, found: false };
                i.block.stmts.iter_mut().for_each(|stmt|{
                    if check_callee.found {

                    } else {
                        check_callee.visit_stmt_mut(stmt);
                    }
                })
            }
        }
    }
}

struct RefBorrower<'a> {
    fn_name: &'a str,
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

fn make_borrows(file_name: &str, new_file_name: &str, callee_name: &str, caller_name: &str) {

}