use quote::ToTokens;

use std::fs;

use syn::punctuated::Punctuated;
use syn::{visit_mut::VisitMut, Expr, ExprAssign, ExprAssignOp, ExprCall, ExprMethodCall, FnArg, ItemFn, Local, Macro, Pat, Token, Type, TypeReference};
use utils::format_source;

struct RefBorrowAssignerHelper<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

fn visit_sub_expr_find_id<V>(v: &mut V, node: &mut Expr)
where
    V: VisitMut,
{
    match node {
        Expr::Array(e) => v.visit_expr_array_mut(e),
        Expr::Assign(e) => v.visit_expr_assign_mut(e),
        Expr::AssignOp(e) => v.visit_expr_assign_op_mut(e),
        Expr::Async(e) => v.visit_expr_async_mut(e),
        Expr::Await(e) => v.visit_expr_await_mut(e),
        Expr::Binary(e) => v.visit_expr_binary_mut(e),
        Expr::Block(e) => v.visit_expr_block_mut(e),
        Expr::Box(e) => v.visit_expr_box_mut(e),
        Expr::Break(e) => v.visit_expr_break_mut(e),
        Expr::Call(e) => v.visit_expr_call_mut(e),
        Expr::Cast(e) => v.visit_expr_cast_mut(e),
        Expr::Closure(e) => v.visit_expr_closure_mut(e),
        Expr::Continue(e) => v.visit_expr_continue_mut(e),
        Expr::Field(e) => v.visit_expr_field_mut(e),
        Expr::ForLoop(e) => v.visit_expr_for_loop_mut(e),
        Expr::Group(e) => v.visit_expr_group_mut(e),
        Expr::If(e) => v.visit_expr_if_mut(e),
        Expr::Index(e) => v.visit_expr_index_mut(e),
        Expr::Let(e) => v.visit_expr_let_mut(e),
        Expr::Loop(e) => v.visit_expr_loop_mut(e),
        Expr::Macro(e) => v.visit_expr_macro_mut(e),
        Expr::Match(e) => v.visit_expr_match_mut(e),
        Expr::MethodCall(e) => v.visit_expr_method_call_mut(e),
        Expr::Paren(e) => v.visit_expr_paren_mut(e),
        Expr::Path(e) => v.visit_expr_path_mut(e),
        Expr::Range(e) => v.visit_expr_range_mut(e),
        Expr::Reference(e) => v.visit_expr_reference_mut(e),
        Expr::Repeat(e) => v.visit_expr_repeat_mut(e),
        Expr::Return(e) => v.visit_expr_return_mut(e),
        Expr::Struct(e) => v.visit_expr_struct_mut(e),
        Expr::Try(e) => v.visit_expr_try_mut(e),
        Expr::TryBlock(e) => v.visit_expr_try_block_mut(e),
        Expr::Tuple(e) => v.visit_expr_tuple_mut(e),
        Expr::Type(e) => v.visit_expr_type_mut(e),
        Expr::Unary(e) => v.visit_expr_unary_mut(e),
        Expr::While(e) => v.visit_expr_while_mut(e),
        Expr::Yield(e) => v.visit_expr_yield_mut(e),
        _ => (),
    }
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
                false => visit_sub_expr_find_id(self, i),
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

struct CalleeInputs<'a> {
    fn_name: &'a str,
    inputs: &'a mut Vec<String>,
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
            false => visit_sub_expr_find_id(self, i),
        }
    }
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        println!("expression call: {}", i.clone().into_token_stream().to_string());
        println!("func call: {}", id.as_str());
        match id == self.callee_fn_name {
            false => (),
            true => self.found = true,
        }
    }

    fn visit_local_mut(&mut self, i: &mut Local) {
        match &mut i.pat {
            Pat::Ident(id) => match &id.mutability {
                None => (),
                Some(_) => {
                    self.decl_mut.push(id.ident.to_string());
                }
            },
            _ => (),
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
            false => visit_sub_expr_find_id(self, i),
        }
    }

    fn visit_macro_mut(&mut self, i: &mut Macro) {
        // only support *print*! macros as it is most common
        let path = i.path.clone().into_token_stream().to_string();
        match path.contains("print") {
            false => (),
            true => {
                println!("visiting macro:{}", i.clone().into_token_stream().to_string());
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
            false => {
                visit_sub_expr_find_id(self, i);
            }
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
            true => {
                self.mut_methods.clone().iter().for_each(|mut_call| {
                    let mut_call_id = mut_call.receiver.as_ref().into_token_stream().to_string();
                    if i.clone().method == mut_call.method && id == mut_call_id {
                        self.make_mut.push(id.clone())
                    }
                })
            },
            false => (),
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
            false => (),
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
    println!("{}", mut_method_call_expr_file);
    let mut_methods_content: String = fs::read_to_string(&mut_method_call_expr_file).unwrap().parse().unwrap();
    let mut mut_methods = vec![];
    for call in mut_methods_content.split("\n") {
        match syn::parse_str::<syn::ExprMethodCall>(call)
            .map_err(|e| format!("{:?}", e)) {
            Ok(call) => mut_methods.push(call),
            Err(_) => (),
        }
    }

    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut callee_inputs = vec![];
    let mut callee_input_helper = CalleeInputs {
        fn_name: callee_fn_name,
        inputs: &mut callee_inputs,
    };
    callee_input_helper.visit_file_mut(&mut file);
    let mut make_ref = vec![];
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
