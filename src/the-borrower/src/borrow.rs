use std::borrow::Borrow;
use quote::{ToTokens};
use std::fs;
use syn::{visit_mut::VisitMut, Expr, ExprAssign, ExprCall, FnArg, ItemFn, Type, TypeReference, Attribute, Macro, ExprMacro};
use utils::format_source;

struct RefBorrowAssignerHelper<'a> {
    make_ref: &'a Vec<String>,
    make_mut: &'a Vec<String>,
}

fn visit_sub_expr_find_id<V>(v: &mut V, node: &mut Expr) where V : VisitMut {
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
        Expr::Macro(e) => {println!("visitng macro: {}", e.clone().into_token_stream().to_string()); v.visit_expr_macro_mut(e)},
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
        match self.make_mut.contains(&id) || self.make_ref.contains(&id) {
            true => *i = syn::parse_quote!{*#i},
            false => visit_sub_expr_find_id(self, i),

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
                            mutability: (Some(syn::parse_quote!{mut})),
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
                    make_mut: &self.make_mut,
                };
                i.sig.inputs.iter_mut().for_each(|fn_arg| borrow_assigner.visit_fn_arg_mut(fn_arg));
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
                i.sig.inputs.iter().for_each(|fn_arg| {
                    match fn_arg {
                        FnArg::Receiver(_) => (),
                        FnArg::Typed(t) => {
                            self.inputs.push(t.pat.as_ref().into_token_stream().to_string())
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
                i.tokens.to_string().split(",").into_iter().for_each(|x| {
                    let id = x.trim().to_string();
                    match self.input.contains(&id) {
                        true => self.make_ref.push(id),
                        false => {}
                    }
                })
            }
        }
    }
}

struct CallerHelper<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    callee_inputs: &'a Vec<String>,
    make_ref: &'a mut Vec<String>, // must be ref (not deciding whether immutable/mut yet
}

impl VisitMut for CallerHelper<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                let mut check_callee = CallerCheckCallee {
                    callee_fn_name: self.callee_fn_name,
                    found: false,
                };
                let mut check_input = CallerCheckInput {
                    input: &self.callee_inputs,
                    make_ref: &mut self.make_ref,
                };
                self.callee_inputs.iter().for_each(|x| println!("inputs: {}", x));
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
            true => {
                i.args.iter_mut().for_each(|arg| {
                    let id = arg.into_token_stream().to_string();
                    match self.make_mut.contains(&id) {
                        true => {
                            *arg = syn::parse_quote!{&mut #arg};
                        }
                        false => {
                            match self.make_ref.contains(&id) {
                                false => (),
                                true => {
                                    *arg = syn::parse_quote!{&#arg};
                                }
                            }
                        }
                    }
                })
            }
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
                let mut helper = CallerFnArgHelper{
                    callee_fn_name: self.callee_fn_name,
                    make_ref: self.make_ref,
                    make_mut: self.make_mut,
                };
                i.block.stmts.iter_mut().for_each(|stmt| helper.visit_stmt_mut(stmt))
            }
        }
    }
}

pub fn make_borrows(file_name: &str, new_file_name: &str, callee_fn_name: &str, caller_fn_name: &str) {
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut callee_inputs = vec![];
    let mut callee_input_helper = CalleeInputs { fn_name: callee_fn_name, inputs: &mut callee_inputs };
    callee_input_helper.visit_file_mut(&mut file);
    let mut make_ref = vec![];
    let mut caller_helper = CallerHelper {
        caller_fn_name,
        callee_fn_name,
        callee_inputs: &callee_inputs,
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
        make_mut: &make_mut
    };
    caller_assigner.visit_file_mut(&mut file);
    let file = file.into_token_stream().to_string();
    fs::write(new_file_name.to_string(), format_source(&file)).unwrap()
}
