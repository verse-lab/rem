
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

struct CheckCalleeWithinLoopHelper<'a> {
    callee_fn_name: &'a str,
    callee_in_loop: bool,
}

impl VisitMut for CheckCalleeWithinLoopHelper<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        match id == self.callee_fn_name {
            true => self.callee_in_loop = true,
            false => syn::visit_mut::visit_expr_call_mut(self, i),
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

            _ => syn::visit_mut::visit_expr_mut(self, i),
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

            _ => syn::visit_mut::visit_expr_mut(self, i),
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
            _ => syn::visit_mut::visit_stmt_mut(self, i),
        }
    }
}

struct MakeBrkAndContVisitor<'a> {
    callee_fn_name: &'a str,
    success: bool,
}

impl VisitMut for MakeBrkAndContVisitor<'_> {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        println!(
            "expr make brk: {}",
            i.clone().into_token_stream().to_string()
        );
        match i {
            Expr::Break(e) => {
                match &e.expr {
                    None => {}
                    Some(_) => self.success = false,
                }
                let new_e_str = format!(
                    "return {}{}::Break",
                    ENUM_NAME,
                    make_pascal_case(self.callee_fn_name)
                );
                let new_e: Expr = syn::parse_str(new_e_str.as_str()).unwrap();
                *i = new_e
            }
            Expr::Continue(_) => {
                let new_e_str = format!(
                    "return {}{}::Continue",
                    ENUM_NAME,
                    make_pascal_case(self.callee_fn_name)
                );
                let new_e: Expr = syn::parse_str(new_e_str.as_str()).unwrap();
                *i = new_e
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
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

                let mut helper = MakeBrkAndContVisitor {
                    callee_fn_name: self.callee_fn_name,
                    success: self.success,
                };
                helper.visit_block_mut(i.block.as_mut());
                self.success = helper.success;

                let ok = quote!(Ok);
                match i.block.stmts.last_mut() {
                    None => {}
                    Some(s) => match s {
                        Stmt::Expr(_) => {
                            let mut helper = MakeLastReturnBlkVisitor {};
                            helper.visit_stmt_mut(s);
                            let re = quote!(result);
                            let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(#re)};
                            i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                        }
                        _ => {
                            let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(())};
                            i.block.stmts.push(Stmt::Expr(ret_stmt_expr))
                        }
                    },
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
                match i.block.stmts.last_mut() {
                    None => {}
                    Some(s) => {
                        println!("last stmt: {}", s.into_token_stream().to_string());
                        match s {
                            Stmt::Expr(_) => {
                                let mut helper = MakeLastReturnBlkVisitor {};
                                helper.visit_stmt_mut(s);
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
                    false => syn::visit_mut::visit_expr_mut(self, i),
                }
            }
            _ => syn::visit_mut::visit_expr_mut(self, i),
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
            println!(
                "has break {} or cont {}",
                callee_visitor.has_break, callee_visitor.has_continue
            );
            let mut make_brk_and_cont = MakeBrkAndCont {
                callee_fn_name,
                success,
            };
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
