use std::fs;

use convert_case::{Case, Casing};
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::visit_mut::VisitMut;
use syn::{Expr, ExprCall, ExprMatch, ExprReturn, Item, ItemEnum, ItemFn, ReturnType, Stmt, Type};
use utils::format_source;

const ENUM_NAME: &str = "Ret";

fn make_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

fn visit_subexpr<V>(v: &mut V, node: &mut Expr)
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

struct CheckCalleeWithinLoopHelper<'a> {
    callee_fn_name: &'a str,
    callee_in_loop: bool,
}

impl VisitMut for CheckCalleeWithinLoopHelper<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        match id == self.callee_fn_name {
            true => self.callee_in_loop = true,
            false => (),
        }
    }
}

struct CheckCalleeWithinLoop<'a> {
    callee_fn_name: &'a str,
    callee_in_loop: bool,
}

impl VisitMut for CheckCalleeWithinLoop<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        let mut helper = CheckCalleeWithinLoopHelper {
            callee_fn_name: self.callee_fn_name,
            callee_in_loop: self.callee_in_loop,
        };
        match i {
            Expr::ForLoop(l) => {
                l.body
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| helper.visit_stmt_mut(stmt));
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }
            Expr::Loop(l) => {
                l.body
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| helper.visit_stmt_mut(stmt));
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }
            Expr::While(l) => {
                l.body
                    .stmts
                    .iter_mut()
                    .for_each(|stmt| helper.visit_stmt_mut(stmt));
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }

            _ => visit_subexpr(self, i),
        }
    }
}

struct CallerVisitor<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    callee_in_loop: bool,
    // very simplified handling: if caller has loop and callee has break/continue but no loop
    // assume it's control flow for caller otherwise, keep the same (assume control for callee loop)
    caller_rety: &'a mut ReturnType,
}

impl VisitMut for CallerVisitor<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.clone().to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {
                *self.caller_rety = i.sig.output.clone();
                let mut helper = CheckCalleeWithinLoop {
                    callee_fn_name: self.callee_fn_name,
                    callee_in_loop: false,
                };
                helper.visit_item_fn_mut(i);
                self.callee_in_loop = helper.callee_in_loop;
            }
        }
    }
}

struct CalleeCheckReturn {
    has_return: bool,
}

impl VisitMut for CalleeCheckReturn {
    fn visit_expr_return_mut(&mut self, _: &mut ExprReturn) {
        self.has_return = true
    }
}

struct CalleeCheckLoops {
    has_break: bool,
    has_continue: bool,
}

impl VisitMut for CalleeCheckLoops {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Break(_) => self.has_break = true,
            Expr::Continue(_) => self.has_continue = true,

            // don't check for loop control within callee loops
            Expr::ForLoop(_) => (),
            Expr::Loop(_) => (),
            Expr::While(_) => (),

            _ => visit_subexpr(self, i),
        }
    }
}

struct CalleeCheckNCF<'a> {
    callee_fn_name: &'a str,
    within_caller_loop: bool,
    has_break: bool,
    has_continue: bool,
    has_return: bool,
}

impl VisitMut for CalleeCheckNCF<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.callee_fn_name {
            false => (),
            true => {
                let mut check_return = CalleeCheckReturn {
                    has_return: self.has_return,
                };
                let mut check_loops = CalleeCheckLoops {
                    has_break: self.has_break,
                    has_continue: self.has_continue,
                };
                i.block.stmts.iter_mut().for_each(|stmt| {
                    check_return.visit_stmt_mut(stmt);
                    if self.within_caller_loop {
                        check_loops.visit_stmt_mut(stmt);
                    }
                });
                self.has_return = check_return.has_return;
                self.has_break = check_loops.has_break;
                self.has_continue = check_loops.has_continue;
            }
        }
    }
}

struct MakeLastReturnBlkVisitor {}

impl VisitMut for MakeLastReturnBlkVisitor {
    fn visit_stmt_mut(&mut self, i: &mut Stmt) {
        match i {
            Stmt::Expr(e) => {
                let re = quote!(result);
                let e = e.clone();
                *i = syn::parse_quote! {let #re = #e;}
            }
            _ => (),
        }
    }
}

struct MakeBrkAndContVisitor<'a> {
    callee_fn_name: &'a str,
    success: bool,
}

impl VisitMut for MakeBrkAndContVisitor<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        println!("expr make brk: {}", i.clone().into_token_stream().to_string());
        match i {
            Expr::Break(e) => {
                match &e.expr {
                    None => {}
                    Some(_) => self.success = false,
                }
                let new_e_str = format!("return {}{}::Break", ENUM_NAME, make_pascal_case(self.callee_fn_name));
                let new_e : Expr = syn::parse_str(new_e_str.as_str()).unwrap();
                *i = new_e
            }
            Expr::Continue(_) => {
                let new_e_str = format!("return {}{}::Continue", ENUM_NAME, make_pascal_case(self.callee_fn_name));
                let new_e : Expr = syn::parse_str(new_e_str.as_str()).unwrap();
                *i = new_e
            }
            _ => visit_subexpr(self, i),
        }
    }
}

struct MakeBrkAndCont<'a> {
    callee_fn_name: &'a str,
    success: bool,
}

impl VisitMut for MakeBrkAndCont<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.callee_fn_name {
            false => (),
            true => {
                let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(self.callee_fn_name));
                let ident = Ident::new(ident_str.as_str(), Span::call_site());
                let callee_rety = match i.sig.output.clone() {
                    ReturnType::Default => Type::Verbatim(quote! {()}),
                    ReturnType::Type(_, t) => t.as_ref().clone(),
                };
                let ty: Type = Type::Verbatim(quote! {#ident<#callee_rety>});
                i.sig.output = ReturnType::Type(syn::parse_quote! {->}, Box::new(ty));

                let mut helper = MakeBrkAndContVisitor { callee_fn_name: self.callee_fn_name, success: self.success };
                helper.visit_block_mut(i.block.as_mut());
                self.success = helper.success;

                let ok = quote!(Ok);
                match i.block.stmts.last() {
                    None => {}
                    Some(s) => {
                        match s {
                            Stmt::Expr(_) => {
                                let mut helper = MakeLastReturnBlkVisitor {};
                                helper.visit_block_mut(i.block.as_mut());
                                let re = quote!(result);
                                let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(#re)};
                                i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                            }
                            _ => {
                                let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(())};
                                i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                            }
                        }
                    }
                }
            }
        }
    }
}

struct MakeReturn<'a> {
    callee_fn_name: &'a str,
    caller_rety: &'a Type,
}

impl VisitMut for MakeReturn<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.callee_fn_name {
            false => (),
            true => {
                let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(self.callee_fn_name));
                let ident = Ident::new(ident_str.as_str(), Span::call_site());
                let caller_rety = self.caller_rety.clone();
                let callee_rety = match i.sig.output.clone() {
                    ReturnType::Default => Type::Verbatim(quote! {()}),
                    ReturnType::Type(_, t) => t.as_ref().clone(),
                };
                let ty: Type = Type::Verbatim(quote! {#ident<#callee_rety,#caller_rety>});
                i.sig.output = ReturnType::Type(syn::parse_quote! {->}, Box::new(ty));

                let ok = quote!(Ok);
                match i.block.stmts.last() {
                    None => {}
                    Some(s) => {
                        println!("last stmt: {}", s.into_token_stream().to_string());
                        match s {
                            Stmt::Expr(_) => {
                                let mut helper = MakeLastReturnBlkVisitor {};
                                helper.visit_block_mut(i.block.as_mut());
                                let re = quote!(result);
                                let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(#re)};
                                i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                            }
                            _ => {
                                let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(())};
                                i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                            }
                        }
                    }
                }

            }
        }
    }
}

struct MakeCallerReturnHelper<'a> {
    callee_fn_name: &'a str,
}
impl VisitMut for MakeCallerReturnHelper<'_> {
    fn visit_expr_return_mut(&mut self, i: &mut ExprReturn) {
        let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(self.callee_fn_name));
        let ident = Ident::new(ident_str.as_str(), Span::call_site());
        let return_t = quote! {Return};
        match i.expr.clone() {
            None => {
                let rety: Expr = syn::parse_quote! {#ident::#return_t(())};
                i.expr = Some(Box::new(rety))
            }
            Some(e) => {
                let e = e.as_ref().clone();
                let rety: Expr = syn::parse_quote! {#ident::#return_t(#e)};
                i.expr = Some(Box::new(rety));
            }
        }
    }
}

struct MakeCallerReturn<'a> {
    callee_fn_name: &'a str,
}

impl VisitMut for MakeCallerReturn<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.callee_fn_name {
            false => (),
            true => {
                let mut helper = MakeCallerReturnHelper {
                    callee_fn_name: self.callee_fn_name,
                };
                helper.visit_item_fn_mut(i)
            }
        }
    }
}

struct MatchCallSiteHelper<'a> {
    callee_fn_name: &'a str,
    has_return: bool,
    has_continue: bool,
    has_break: bool,
}

impl VisitMut for MatchCallSiteHelper<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        println!("visit expr: {}", i.into_token_stream().to_string());
        match i {
            Expr::Call(c) => {
                let id = c.func.clone().as_ref().into_token_stream().to_string();
                match id == self.callee_fn_name {
                    true => {
                        let e = i.clone().into_token_stream().to_string();
                        let enum_name_fn = make_pascal_case(self.callee_fn_name);
                        let match_str = format!(
                            "match {} {{\n{} {} {} {}\n}}",
                            e,
                            format!("{}{}::Ok(x) => x,\n", ENUM_NAME, enum_name_fn),
                            if self.has_return {
                                format!("{}{}::Return(x) => return x,\n", ENUM_NAME, enum_name_fn)
                            } else {
                                "".to_string()
                            },
                            if self.has_break {
                                format!("{}{}::Break => break,\n", ENUM_NAME, enum_name_fn)
                            } else {
                                "".to_string()
                            },
                            if self.has_continue {
                                format!("{}{}::Continue => continue,", ENUM_NAME, enum_name_fn)
                            } else {
                                "".to_string()
                            },
                        );
                        let match_expr: ExprMatch = syn::parse_str(match_str.as_str()).unwrap();
                        *i = Expr::Match(match_expr)
                    }
                    false => visit_subexpr(self, i),
                }
            }
            _ => visit_subexpr(self, i),
        }
    }
}

struct MatchCallSite<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
    has_return: bool,
    has_continue: bool,
    has_break: bool,
}

impl VisitMut for MatchCallSite<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            true => {
                let mut helper = MatchCallSiteHelper {
                    callee_fn_name: self.callee_fn_name,
                    has_return: self.has_return,
                    has_continue: self.has_continue,
                    has_break: self.has_break,
                };
                helper.visit_item_fn_mut(i)
            }
            false => {}
        }
    }
}

pub fn make_controls(
    file_name: &str,
    new_file_name: &str,
    callee_fn_name: &str,
    caller_fn_name: &str,
) -> bool {
    let mut success = true;
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut caller_rety = ReturnType::Default;
    let mut caller_visitor = CallerVisitor {
        caller_fn_name,
        callee_fn_name,
        callee_in_loop: false,
        caller_rety: &mut caller_rety,
    };
    caller_visitor.visit_file_mut(&mut file);

    let mut callee_visitor = CalleeCheckNCF {
        callee_fn_name,
        within_caller_loop: caller_visitor.callee_in_loop,
        has_break: false,
        has_continue: false,
        has_return: false,
    };
    callee_visitor.visit_file_mut(&mut file);

    if callee_visitor.has_return || callee_visitor.has_continue || callee_visitor.has_break {
        let caller_rety = match caller_visitor.caller_rety {
            ReturnType::Default => Type::Verbatim(quote! {()}),
            ReturnType::Type(_, t) => t.as_ref().clone(),
        };

        if callee_visitor.has_return {
            let mut make_ret = MakeReturn {
                callee_fn_name,
                caller_rety: &caller_rety,
            };
            make_ret.visit_file_mut(&mut file);

            let mut make_caller_ret = MakeCallerReturn { callee_fn_name };
            make_caller_ret.visit_file_mut(&mut file);
        }

        if callee_visitor.has_break || callee_visitor.has_continue {
            println!("has break {} or cont {}", callee_visitor.has_break, callee_visitor.has_continue);
            let mut make_brk_and_cont = MakeBrkAndCont { callee_fn_name, success };
            make_brk_and_cont.visit_file_mut(&mut file);
            success = make_brk_and_cont.success
        }

        let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(callee_fn_name));
        let enum_str = format!(
            "enum {}<A{}> \n{{Ok(A),\n{}{}{}}}",
            ident_str,
            if callee_visitor.has_return { ", B" } else { "" },
            if callee_visitor.has_return {
                "Return(B),\n"
            } else {
                ""
            },
            if callee_visitor.has_break {
                "Break,\n"
            } else {
                ""
            },
            if callee_visitor.has_continue {
                "Continue,\n"
            } else {
                ""
            },
        );
        let enum_ret: ItemEnum = syn::parse_str(enum_str.as_str()).unwrap();
        file.items.push(Item::Enum(enum_ret));

        let mut caller_matcher = MatchCallSite {
            caller_fn_name,
            callee_fn_name,
            has_return: callee_visitor.has_return,
            has_continue: callee_visitor.has_continue,
            has_break: callee_visitor.has_break,
        };
        caller_matcher.visit_file_mut(&mut file);
    }

    let file = file.into_token_stream().to_string();
    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
    success
}
