use proc_macro2::Ident;
use quote::ToTokens;
use std::fs;
use syn::{visit_mut::VisitMut, Expr, ExprAssign, ExprCall, FnArg, ItemFn, Type, TypeReference};
use utils::format_source;

struct RefBorrowAssignerHelper<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for RefBorrowAssignerHelper<'_> {
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
                    }
                    false => match self.make_mut.contains(&id) {
                        false => (),
                        true => {
                            t.ty = Box::from(Type::Reference(TypeReference {
                                and_token: Default::default(),
                                lifetime: None,
                                mutability: None,
                                elem: t.ty.clone(),
                            }))
                        }
                    },
                }
            }
        }
    }
}

struct RefBorrowAssigner<'a> {
    fn_name: &'a str,
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for RefBorrowAssigner<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            false => (),
            true => {
                let mut borrow_assigner = RefBorrowAssignerHelper {
                    make_ref: self.make_ref,
                    make_mut: &self.make_mut,
                };
                i.block
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| borrow_assigner.visit_stmt_mut(stmt))
            }
        }
    }
}

struct CallerCheckCallee<'a> {
    callee_fn_name: &'a str,
    found: bool,
}

impl VisitMut for CallerCheckCallee<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        match id == self.callee_fn_name {
            false => (),
            true => self.found = true,
        }
    }
}

struct CallerCheckInput<'a> {
    input: &'a Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CallerCheckInput<'_> {
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        let id = i.into_token_stream().to_string();
        match self.input.contains(&id) {
            false => (),
            true => self.make_ref.push(id),
        }
    }
}

struct CallerHelper<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    make_ref: &'a mut Vec<String>, // must be ref (not deciding whether immut/mut yet
}

impl VisitMut for CallerHelper<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                let inputs = i.sig.inputs.iter().cloned();
                let inputs_str: Vec<String> = inputs
                    .map(|fn_arg| fn_arg.into_token_stream().to_string())
                    .collect();
                let mut check_callee = CallerCheckCallee {
                    callee_fn_name: self.callee_fn_name,
                    found: false,
                };
                let mut make_ref = vec![];
                let mut check_input = CallerCheckInput {
                    input: &inputs_str,
                    make_ref: &mut make_ref,
                };
                i.block.stmts.iter_mut().for_each(|stmt| {
                    if check_callee.found {
                        check_input.visit_stmt_mut(stmt);
                    } else {
                        check_callee.visit_stmt_mut(stmt);
                    }
                })
            }
        }
    }
}

struct MutableBorrowerHelper<'a> {
    make_ref: &'a mut Vec<String>,
    make_mut: &'a mut Vec<String>,
}

impl VisitMut for MutableBorrowerHelper<'_> {
    fn visit_expr_assign_mut(&mut self, i: &mut ExprAssign) {
        let id = i.left.clone().into_token_stream().to_string();
        match self.make_ref.contains(&id) {
            false => (),
            true => self.make_mut.push(id),
        }
    }
}

struct MutableBorrower<'a> {
    fn_name: &'a str,
    make_ref: &'a mut Vec<String>,
    make_mut: &'a mut Vec<String>,
}

impl VisitMut for MutableBorrower<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            false => (),
            true => {
                let mut mut_borrower_helper = MutableBorrowerHelper {
                    make_ref: self.make_ref,
                    make_mut: self.make_mut,
                };
                i.block
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| mut_borrower_helper.visit_stmt_mut(stmt))
            }
        }
    }
}

fn make_borrows(file_name: &str, new_file_name: &str, callee_fn_name: &str, caller_fn_name: &str) {
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut make_ref = vec![];
    let mut caller_helper = CallerHelper {
        caller_fn_name,
        callee_fn_name,
        make_ref: &mut make_ref,
    };
    caller_helper.visit_file_mut(&mut file);
    let mut make_mut = vec![];
    let mut mut_borrower = MutableBorrower {
        fn_name: callee_fn_name,
        make_ref: &mut make_ref,
        make_mut: &mut make_mut,
    };
    mut_borrower.visit_file_mut(&mut file);
    let mut assigner = RefBorrowAssigner {
        fn_name: callee_fn_name,
        make_ref: &make_ref,
        make_mut: &make_mut,
    };
    assigner.visit_file_mut(&mut file);
    let file = file.into_token_stream().to_string();
    fs::write(new_file_name.to_string(), format_source(&file)).unwrap()
}
