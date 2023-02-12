use quote::ToTokens;

use std::fs;
use proc_macro2::Ident;

use syn::punctuated::Punctuated;
use syn::{visit_mut::VisitMut, Expr, ExprAssign, ExprAssignOp, ExprCall, ExprMethodCall, FnArg, ItemFn, Local, Macro, Pat, Token, Type, TypeReference, ExprReturn, Stmt};
use utils::format_source;

struct RefBorrowAssignerHelper<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for RefBorrowAssignerHelper<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let id = i.into_token_stream().to_string();
        println!("id expr: {}", &id);
        match i {
            //no need to star method call left side but need to for args
            Expr::MethodCall(e) => {
                e.args.iter_mut().for_each(|el| self.visit_expr_mut(el));
            }
            //no starring index lhs but need to star its index
            Expr::Index(e) => {
                self.visit_expr_mut(e.index.as_mut());
            }
            //no need to star let binding lhs
            Expr::Let(e) => {
                self.visit_expr_mut(e.expr.as_mut());
            }
            _ => match self.make_mut.contains(&id) || self.make_ref.contains(&id) {
                true => *i = syn::parse_quote! {*#i},
                false => syn::visit_mut::visit_expr_mut(self, i),
            },
        }
    }

    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (),
            FnArg::Typed(t) => {
                let id = t.pat.as_ref().into_token_stream().to_string();
                match self.make_mut.contains(&id) {
                    true => {
                        t.ty = Box::from(Type::Reference(TypeReference {
                            and_token: Default::default(),
                            lifetime: None,
                            mutability: (Some(syn::parse_quote! {mut})),
                            elem: t.ty.clone(),
                        }))
                    }
                    false => match self.make_ref.contains(&id) {
                        false => syn::visit_mut::visit_fn_arg_mut(self, i),
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

struct CalleeBorrowAssigner<'a> {
    fn_name: &'a str,
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for CalleeBorrowAssigner<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            false => (),
            true => {
                let mut borrow_assigner = RefBorrowAssignerHelper {
                    make_ref: self.make_ref,
                    make_mut: self.make_mut,
                };
                i.sig
                    .inputs
                    .iter_mut()
                    .for_each(|fn_arg| borrow_assigner.visit_fn_arg_mut(fn_arg));
                i.block
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| borrow_assigner.visit_stmt_mut(stmt))
            }
        }
    }
}

struct IdentHelper<'a> {
    idents: &'a mut Vec<String>,
}

impl VisitMut for IdentHelper<'_>{
    fn visit_ident_mut(&mut self, i: &mut Ident) {
        self.idents.push(i.to_string());
        syn::visit_mut::visit_ident_mut(self, i)
    }
}

struct CalleeExprHelper<'a> {
    inputs: &'a Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CalleeExprHelper<'_>{
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Reference(r)=> {
                let mut idents = vec![];
                let mut ident_helper = IdentHelper{ idents: &mut idents };
                ident_helper.visit_expr_mut(r.expr.as_mut());
                for id in idents {
                    if self.inputs.contains(&id) {
                        self.make_ref.push(id);
                    }
                }
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
}

struct CalleeReturnsHelper<'a> {
    inputs: &'a Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CalleeReturnsHelper<'_>{
    fn visit_expr_return_mut(&mut self, i: &mut ExprReturn) {
        match &mut i.expr {
            None => {}
            Some(e) => {
                let mut expr_helper = CalleeExprHelper { inputs: self.inputs, make_ref: self.make_ref };
                expr_helper.visit_expr_mut(e)
            }
        }
    }

    fn visit_stmt_mut(&mut self, i: &mut Stmt) {
        match i {
            Stmt::Expr(e) => {
                let mut expr_helper = CalleeExprHelper { inputs: self.inputs, make_ref: self.make_ref };
                expr_helper.visit_expr_mut(e)
            }
            _ => syn::visit_mut::visit_stmt_mut(self, i),
        }
    }
}

struct CalleeInputs<'a> {
    fn_name: &'a str,
    inputs: &'a mut Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CalleeInputs<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name {
            true => {
                i.sig.inputs.iter().for_each(|fn_arg| match fn_arg {
                    FnArg::Receiver(_) => (),
                    FnArg::Typed(t) => {
                        match t.ty.as_ref() {
                            Type::Reference(_) => (), // don't add reference no need to make it a ref
                            _ => self
                                .inputs
                                .push(t.pat.as_ref().into_token_stream().to_string()),
                        }
                    }
                });
            }
            false => (),
        }
        let mut ret_helper = CalleeReturnsHelper { inputs: self.inputs, make_ref: self.make_ref };
        ret_helper.visit_item_fn_mut(i)
    }
}

struct CallerCheckCallee<'a> {
    callee_fn_name: &'a str,
    decl_mut: &'a mut Vec<String>,
    found: bool,
    check_input_visitor: &'a mut CallerCheckInput<'a>,
}

impl VisitMut for CallerCheckCallee<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match self.found {
            true => self.check_input_visitor.visit_expr_mut(i),
            false => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        println!(
            "expression call: {}",
            i.clone().into_token_stream().to_string()
        );
        println!("func call: {}", id.as_str());
        match id == self.callee_fn_name {
            false => syn::visit_mut::visit_expr_call_mut(self, i),
            true => self.found = true,
        }
    }

    fn visit_local_mut(&mut self, i: &mut Local) {
        println!("decl mut: {}", i.clone().into_token_stream().to_string());
        match &mut i.pat {
            Pat::Ident(id) => match &id.mutability {
                None => (),
                Some(_) => {
                    println!("decl mut: {}", id.ident.clone().into_token_stream().to_string());
                    self.decl_mut.push(id.ident.clone().into_token_stream().to_string());
                }
            },
            Pat::Type(t) => {
                match t.pat.as_ref() {
                    Pat::Ident(id) => match id.mutability {
                        None => (),
                        Some(_) => {
                            println!("decl mut: {}", id.ident.clone().into_token_stream().to_string());
                            self.decl_mut.push(id.ident.clone().into_token_stream().to_string());
                        }
                    },
                    _ => syn::visit_mut::visit_local_mut(self, i),
                }
            }
            _ => syn::visit_mut::visit_local_mut(self, i),
        };
    }
}

struct CallerCheckInput<'a> {
    input: &'a Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for CallerCheckInput<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let id = i.into_token_stream().to_string();
        println!("id: {}, in inputs: {}", &id, self.input.contains(&id));
        match self.input.contains(&id) {
            true => self.make_ref.push(id),
            false => syn::visit_mut::visit_expr_mut(self, i),
        }
    }

    fn visit_macro_mut(&mut self, i: &mut Macro) {
        // only support *print*! macros as it is most common
        let path = i.path.clone().into_token_stream().to_string();
        match path.contains("print") {
            false => syn::visit_mut::visit_macro_mut(self, i),
            true => {
                println!(
                    "visiting macro:{}",
                    i.clone().into_token_stream().to_string()
                );
                let tokens = i.tokens.clone();
                let mut expr_punc: Punctuated<Expr, Token!(,)> = syn::parse_quote! {#tokens};
                expr_punc.iter_mut().for_each(|e| self.visit_expr_mut(e));
            }
        }
    }
}

struct CallerHelper<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    callee_inputs: &'a Vec<String>,
    decl_mut: &'a mut Vec<String>,
    make_ref: &'a mut Vec<String>, // must be ref (not deciding whether immutable/mut yet
}

impl VisitMut for CallerHelper<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                i.sig.inputs.clone().iter().for_each(|input| match input {
                    FnArg::Receiver(_) => (),
                    FnArg::Typed(t) => match t.ty.as_ref() {
                        Type::Reference(r) => match r.mutability {
                            None => (),
                            Some(_) => self
                                .decl_mut
                                .push(t.pat.as_ref().into_token_stream().to_string()),
                        },
                        _ => (),
                    },
                });

                let mut check_input = CallerCheckInput {
                    input: &self.callee_inputs,
                    make_ref: &mut self.make_ref,
                };

                let mut temp = vec![];
                let mut check_input_temp = CallerCheckInput {
                    input: &self.callee_inputs,
                    make_ref: &mut temp,
                };
                let mut check_callee = CallerCheckCallee {
                    callee_fn_name: self.callee_fn_name,
                    decl_mut: self.decl_mut,
                    found: false,
                    check_input_visitor: &mut check_input_temp,
                };

                i.block.stmts.iter_mut().for_each(|stmt| {
                    if check_callee.found {
                        check_input.visit_stmt_mut(stmt);
                    } else {
                        check_callee.visit_stmt_mut(stmt);
                    }
                });

                temp.into_iter().for_each(|x| self.make_ref.push(x))
            }
        }
    }
}

struct MutBorrowLHSChecker<'a> {
    make_mut: &'a mut Vec<String>,
    make_ref: &'a mut Vec<String>,
}

impl VisitMut for MutBorrowLHSChecker<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let id = i.clone().into_token_stream().to_string();
        match self.make_ref.contains(&id) {
            true => self.make_mut.push(id),
            false => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
}

struct MutableBorrowerHelper<'a> {
    make_ref: &'a mut Vec<String>,
    make_mut: &'a mut Vec<String>,
    decl_mut: &'a mut Vec<String>,
    mut_methods: &'a Vec<ExprMethodCall>,
}

impl VisitMut for MutableBorrowerHelper<'_> {
    fn visit_expr_assign_mut(&mut self, i: &mut ExprAssign) {
        let id = i.left.clone().into_token_stream().to_string();
        match self.make_ref.contains(&id) {
            true => self.make_mut.push(id),
            false => {
                let mut lhs_checker = MutBorrowLHSChecker {
                    make_mut: self.make_mut,
                    make_ref: self.make_ref,
                };
                lhs_checker.visit_expr_mut(&mut i.left.clone());
            }
        }
    }

    fn visit_expr_assign_op_mut(&mut self, i: &mut ExprAssignOp) {
        let id = i.left.clone().into_token_stream().to_string();
        match self.make_ref.contains(&id) {
            true => self.make_mut.push(id),
            false => {
                let mut lhs_checker = MutBorrowLHSChecker {
                    make_mut: self.make_mut,
                    make_ref: self.make_ref,
                };
                lhs_checker.visit_expr_mut(&mut i.left.clone());
            }
        }
    }

    // treating all declared mut with method call as mut (cannot look up fn decl)
    fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
        let id = i.receiver.as_ref().into_token_stream().to_string();
        println!(
            "call decl id: {}, {}",
            id,
            i.clone().into_token_stream().to_string()
        );
        match self.decl_mut.contains(&id) {
            true => self.mut_methods.clone().iter().for_each(|mut_call| {
                let mut_call_id = mut_call.receiver.as_ref().into_token_stream().to_string();
                if i.clone().method == mut_call.method && id == mut_call_id {
                    self.make_mut.push(id.clone())
                }
            }),
            false => syn::visit_mut::visit_expr_method_call_mut(self, i),
        }
    }
}

struct MutableBorrower<'a> {
    fn_name: &'a str,
    make_ref: &'a mut Vec<String>,
    make_mut: &'a mut Vec<String>,
    decl_mut: &'a mut Vec<String>,
    mut_methods: &'a Vec<ExprMethodCall>,
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
                    decl_mut: self.decl_mut,
                    mut_methods: self.mut_methods,
                };
                i.block
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| mut_borrower_helper.visit_stmt_mut(stmt))
            }
        }
    }
}

struct CallerFnArgHelper<'a> {
    callee_fn_name: &'a str,
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for CallerFnArgHelper<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let callee = i.func.as_ref().into_token_stream().to_string();
        match callee == self.callee_fn_name {
            false => syn::visit_mut::visit_expr_call_mut(self, i),
            true => i.args.iter_mut().for_each(|arg| {
                let id = arg.into_token_stream().to_string();
                match self.make_mut.contains(&id) {
                    true => {
                        *arg = syn::parse_quote! {&mut #arg};
                    }
                    false => match self.make_ref.contains(&id) {
                        false => (),
                        true => {
                            *arg = syn::parse_quote! {&#arg};
                        }
                    },
                }
            }),
        }
    }
}

struct CallerFnArg<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

impl VisitMut for CallerFnArg<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                let mut helper = CallerFnArgHelper {
                    callee_fn_name: self.callee_fn_name,
                    make_ref: self.make_ref,
                    make_mut: self.make_mut,
                };
                i.block
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| helper.visit_stmt_mut(stmt))
            }
        }
    }
}

pub fn make_borrows(
    file_name: &str,
    new_file_name: &str,
    mut_method_call_expr_file: &str,
    callee_fn_name: &str,
    caller_fn_name: &str,
) {
    let mut_methods_content: String = fs::read_to_string(&mut_method_call_expr_file)
        .unwrap()
        .parse()
        .unwrap();
    let mut mut_methods = vec![];
    for call in mut_methods_content.split("\n") {
        match syn::parse_str::<syn::ExprMethodCall>(call).map_err(|e| format!("{:?}", e)) {
            Ok(call) => mut_methods.push(call),
            Err(_) => (),
        }
    }
    
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut callee_inputs = vec![];
    let mut make_ref = vec![];
    let mut callee_input_helper = CalleeInputs {
        fn_name: callee_fn_name,
        inputs: &mut callee_inputs,
        make_ref: &mut make_ref,
    };
    callee_input_helper.visit_file_mut(&mut file);
    let mut decl_mut = vec![];
    let mut caller_helper = CallerHelper {
        caller_fn_name,
        callee_fn_name,
        callee_inputs: &callee_inputs,
        make_ref: &mut make_ref,
        decl_mut: &mut decl_mut,
    };
    caller_helper.visit_file_mut(&mut file);

    for s in &decl_mut {
        println!("decl {} mut", s);
    }

    let mut make_mut = vec![];
    let mut mut_borrower = MutableBorrower {
        fn_name: callee_fn_name,
        make_ref: &mut make_ref,
        make_mut: &mut make_mut,
        decl_mut: &mut decl_mut,
        mut_methods: &mut_methods,
    };
    mut_borrower.visit_file_mut(&mut file);
    let mut callee_assigner = CalleeBorrowAssigner {
        fn_name: callee_fn_name,
        make_ref: &make_ref,
        make_mut: &make_mut,
    };
    for s in &make_ref {
        println!("make {} ref", s);
    }

    for s in &make_mut {
        println!("make {} mut", s);
    }
    callee_assigner.visit_file_mut(&mut file);

    let mut caller_assigner = CallerFnArg {
        caller_fn_name,
        callee_fn_name,
        make_ref: &make_ref,
        make_mut: &make_mut,
    };
    caller_assigner.visit_file_mut(&mut file);
    let file = file.into_token_stream().to_string();
    fs::write(new_file_name.to_string(), format_source(&file)).unwrap()
}
