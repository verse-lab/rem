use std::fs;

use convert_case::{Case, Casing};
use log::debug;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use rem_utils::{format_source, FindCallee};
use syn::visit_mut::VisitMut;
use syn::{Block, Expr, ExprCall, ExprMatch, ExprMethodCall, ExprReturn, ExprTry, ImplItemMethod, Item, ItemFn, ItemImpl, ItemMod, ItemTrait, ReturnType, Signature, Stmt, TraitItemMethod, Type};
use syn::token::Brace;

const ENUM_NAME: &str = "Ret";

fn make_pascal_case(s: &str) -> String {
    let result = s.to_case(Case::Pascal);
    match result.strip_suffix("ExtractThis") {
        Some(r) => r.to_string(),
        None => result.to_string(),
    }
}

struct CheckCalleeWithinLoopHelper<'a> {
    callee_fn_name: &'a str,
    callee_in_loop: bool,
}

impl VisitMut for CheckCalleeWithinLoopHelper<'_> {
    fn visit_expr_call_mut(&mut self, i: &mut ExprCall) {
        let id = i.func.as_ref().into_token_stream().to_string();
        match id.contains(self.callee_fn_name) {
            true => self.callee_in_loop = true,
            false => syn::visit_mut::visit_expr_call_mut(self, i),
        }
    }

    fn visit_expr_method_call_mut(&mut self, i: &mut ExprMethodCall) {
        let callee = i.clone().method.into_token_stream().to_string();
        match callee.contains(self.callee_fn_name) {
            true => self.callee_in_loop = true,
            false => syn::visit_mut::visit_expr_method_call_mut(self, i),
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
                helper.visit_expr_for_loop_mut(l);
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }
            Expr::Loop(l) => {
                helper.visit_expr_loop_mut(l);
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }
            Expr::While(l) => {
                helper.visit_expr_while_mut(l);
                if helper.callee_in_loop {
                    self.callee_in_loop = true
                };
            }

            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
}

struct CallerVisitor<'a> {
    found: bool,
    caller_fn_name: &'a str,
    callee_finder: &'a mut FindCallee<'a>,
    callee_fn_name: &'a str,
    callee_in_loop: bool,
    // very simplified handling: if caller has loop and callee has break/continue but no loop
    // assume it's control flow for caller otherwise, keep the same (assume control for callee loop)
    caller_rety: &'a mut ReturnType,
}

impl VisitMut for CallerVisitor<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        if self.callee_finder.found {
            return;
        }
        debug!("finding caller in impl...");
        let id = i.sig.ident.clone().to_string();
        match id.contains(self.caller_fn_name) {
            false => (),
            true => {
                debug!("found same id: {}...", id);
                self.callee_finder.visit_impl_item_method_mut(i);
                debug!(
                    "found callee: {}? {}...",
                    self.callee_finder.callee_fn_name, self.callee_finder.found
                );
                if !self.callee_finder.found {
                    return;
                }
                self.caller_visitor(&mut i.sig, &mut i.block)
            }
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        if self.callee_finder.found {
            return;
        }

        let id = i.sig.ident.clone().to_string();
        match id.contains(self.caller_fn_name) {
            false => (),
            true => {
                self.callee_finder.visit_item_fn_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.caller_visitor(&mut i.sig, &mut i.block)
            }
        }
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        if self.callee_finder.found {
            return;
        }

        let id = i.sig.ident.clone().to_string();
        match id.contains(self.caller_fn_name) {
            false => (),
            true => {
                self.callee_finder.visit_trait_item_method_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                let _ = i
                    .default
                    .as_mut()
                    .and_then(|block| Some(self.caller_visitor(&mut i.sig, block)));
            }
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }
}

impl CallerVisitor<'_> {
    fn caller_visitor(&mut self, sig: &mut Signature, block: &mut Block) {
        self.found = true;
        *self.caller_rety = sig.output.clone();
        let mut helper = CheckCalleeWithinLoop {
            callee_fn_name: self.callee_fn_name,
            callee_in_loop: false,
        };
        helper.visit_block_mut(block);
        self.callee_in_loop = helper.callee_in_loop;
    }
}

enum RetTyQMark {
    QMarkOption,
    QMarkResult,
}

struct CalleeDeSugarQMark {
    has_desugared: bool,
    rety_qmark: RetTyQMark,
}

impl VisitMut for CalleeDeSugarQMark {
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        match i {
            Expr::Try(ExprTry { expr, .. }) => {
                let inner = expr.as_mut().clone();
                match self.rety_qmark {
                    RetTyQMark::QMarkOption => {
                        *i = syn::parse_str(
                            format!(
                                "match {} {{ Some(x) => x, None => return None }}",
                                inner.into_token_stream().to_string()
                            )
                            .as_str(),
                        )
                        .unwrap();
                    }
                    RetTyQMark::QMarkResult => {
                        *i = syn::parse_str(
                            format!(
                                "match {} {{ Ok(x) => x, Err(e) => return Err(e) }}",
                                inner.into_token_stream().to_string()
                            )
                            .as_str(),
                        )
                        .unwrap();
                    }
                }
                self.has_desugared = true;
            }
            _ => (),
        }
        syn::visit_mut::visit_expr_mut(self, i);
    }
}

struct CalleeCheckReturn {
    has_return: bool,
}

impl VisitMut for CalleeCheckReturn {
    fn visit_expr_return_mut(&mut self, _e: &mut ExprReturn) {
        debug!("has return?{:?}", _e);
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

#[derive(Debug)]
struct CalleeCheckNCF<'a> {
    found: bool,
    callee_fn_name: &'a str,
    caller_rety: ReturnType,
    within_caller_loop: bool,
    has_break: bool,
    has_continue: bool,
    has_return: bool,
    num_inputs: usize,
}

impl VisitMut for CalleeCheckNCF<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.callee_check_ncf(i.sig.clone(), &mut i.block),
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.callee_check_ncf(i.sig.clone(), &mut i.block),
        }
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => {
                let _ = i
                    .default
                    .as_mut()
                    .and_then(|block| Some(self.callee_check_ncf(i.sig.clone(), block)));
            }
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }
}

impl CalleeCheckNCF<'_> {
    fn callee_check_ncf(&mut self, sig: Signature, block: &mut Block) {
        self.found = true;

        match &self.caller_rety {
            ReturnType::Default => {}
            ReturnType::Type(_, ty) => {
                let mut rety = None;
                if ty
                    .as_ref()
                    .clone()
                    .into_token_stream()
                    .to_string()
                    .starts_with("Result")
                {
                    rety = Some(RetTyQMark::QMarkResult)
                } else if ty
                    .as_ref()
                    .clone()
                    .into_token_stream()
                    .to_string()
                    .starts_with("Option")
                {
                    rety = Some(RetTyQMark::QMarkOption)
                }

                match rety {
                    None => (),
                    Some(rety_qmark) => {
                        debug!("desugaring...");
                        let mut desugar_qmark = CalleeDeSugarQMark {
                            has_desugared: false,
                            rety_qmark,
                        };
                        desugar_qmark.visit_block_mut(block);
                        debug!("desugaring...{}", desugar_qmark.has_desugared);
                        self.has_return = desugar_qmark.has_desugared || self.has_return;
                    }
                }
            }
        }

        self.num_inputs = sig.inputs.len();
        let mut check_return = CalleeCheckReturn {
            has_return: self.has_return,
        };

        let mut check_loops = CalleeCheckLoops {
            has_break: self.has_break,
            has_continue: self.has_continue,
        };
        block.stmts.iter_mut().for_each(|stmt| {
            if !self.has_return {
                check_return.visit_stmt_mut(stmt);
            }
            if self.within_caller_loop {
                check_loops.visit_stmt_mut(stmt);
            }
        });
        self.has_return = check_return.has_return;
        self.has_break = check_loops.has_break;
        self.has_continue = check_loops.has_continue;
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
        // println!(
        //     "expr make brk: {}",
        //     i.clone().into_token_stream().to_string()
        // );
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
    already_did_return: bool,
}

impl VisitMut for MakeBrkAndCont<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.make_brk_and_cont(&mut i.sig, &mut i.block),
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.make_brk_and_cont(&mut i.sig, &mut i.block),
        }
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        let id = i.sig.ident.to_string();
        //println!("caller name: {}, at: {}", self.caller_fn_name, &id);
        match id.contains(self.callee_fn_name) {
            false => (),
            true => {
                let _ = i
                    .default
                    .as_mut()
                    .and_then(|block| Some(self.make_brk_and_cont(&mut i.sig, block)));
            }
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }
}

impl MakeBrkAndCont<'_> {
    fn make_brk_and_cont(&mut self, sig: &mut Signature, block: &mut Block) {
        let mut helper = MakeBrkAndContVisitor {
            callee_fn_name: self.callee_fn_name,
            success: self.success,
        };
        helper.visit_block_mut(block);
        self.success = helper.success;
        if !self.already_did_return {
            let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(self.callee_fn_name));
            let ident = Ident::new(ident_str.as_str(), Span::call_site());
            let callee_rety = match sig.output.clone() {
                ReturnType::Default => Type::Verbatim(quote! {()}),
                ReturnType::Type(_, t) => t.as_ref().clone(),
            };
            let ty: Type = Type::Verbatim(quote! {#ident<#callee_rety>});
            sig.output = ReturnType::Type(syn::parse_quote! {->}, Box::new(ty));

            let ok = quote!(Ok);
            match block.stmts.last_mut() {
                None => {}
                Some(s) => match s {
                    Stmt::Expr(_) => {
                        let mut helper = MakeLastReturnBlkVisitor {};
                        helper.visit_stmt_mut(s);
                        let re = quote!(result);
                        let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(#re)};
                        block.stmts.push(Stmt::Expr(ret_stmt_expr))
                    }
                    _ => {
                        let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(())};
                        block.stmts.push(Stmt::Expr(ret_stmt_expr))
                    }
                },
            }
        }
    }
}

struct MakeReturn<'a> {
    callee_fn_name: &'a str,
    caller_rety: &'a Type,
}

impl VisitMut for MakeReturn<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.make_return(&mut i.sig, &mut i.block),
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        let id = i.sig.ident.to_string();
        //println!("caller name: {}, at: {}", self.caller_fn_name, &id);
        match id.contains(self.callee_fn_name) {
            false => (),
            true => {
                let _ = i
                    .default
                    .as_mut()
                    .and_then(|block| Some(self.make_return(&mut i.sig, block)));
            }
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => self.make_return(&mut i.sig, &mut i.block),
        }
    }
}

impl MakeReturn<'_> {
    fn make_return(&mut self, sig: &mut Signature, block: &mut Block) {
        let ident_str = format!("{}{}", ENUM_NAME, make_pascal_case(self.callee_fn_name));
        let ident = Ident::new(ident_str.as_str(), Span::call_site());
        let caller_rety = self.caller_rety.clone();
        let callee_rety = match sig.output.clone() {
            ReturnType::Default => Type::Verbatim(quote! {()}),
            ReturnType::Type(_, t) => t.as_ref().clone(),
        };
        let ty: Type = Type::Verbatim(quote! {#ident<#callee_rety,#caller_rety>});
        sig.output = ReturnType::Type(syn::parse_quote! {->}, Box::new(ty));

        let ok = quote!(Ok);
        match block.stmts.last_mut() {
            None => {}
            Some(s) => {
                // println!("last stmt: {}", s.into_token_stream().to_string());
                match s {
                    Stmt::Expr(_) => {
                        let mut helper = MakeLastReturnBlkVisitor {};
                        helper.visit_stmt_mut(s);
                        let re = quote!(result);
                        let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(#re)};
                        block.stmts.push(Stmt::Expr(ret_stmt_expr))
                    }
                    _ => {
                        let ret_stmt_expr: Expr = syn::parse_quote! {#ident::#ok(())};
                        block.stmts.push(Stmt::Expr(ret_stmt_expr))
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
    fn visit_expr_mut(&mut self, i: &mut Expr) {
        debug!("expr: {:?}", i);
        syn::visit_mut::visit_expr_mut(self, i);
    }

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
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            true => {
                debug!("found callee: {:?}", i);
                let mut helper = MakeCallerReturnHelper {
                    callee_fn_name: self.callee_fn_name,
                };
                helper.visit_impl_item_method_mut(i)
            }
            false => {}
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            false => (),
            true => {
                debug!("found callee: {:?}", i);
                let mut helper = MakeCallerReturnHelper {
                    callee_fn_name: self.callee_fn_name,
                };
                helper.visit_item_fn_mut(i)
            }
        }
    }
    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        let id = i.sig.ident.to_string();
        match id.contains(self.callee_fn_name) {
            true => {
                debug!("found callee: {:?}", i);
                let mut helper = MakeCallerReturnHelper {
                    callee_fn_name: self.callee_fn_name,
                };
                helper.visit_trait_item_method_mut(i);
            }
            false => {}
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
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
        // println!("visit expr: {}", i.into_token_stream().to_string());
        match i {
            Expr::Call(c) => {
                let id = c.func.clone().as_ref().into_token_stream().to_string();
                match id.contains(self.callee_fn_name) {
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
            // NEED TO FIX TO INCLUDE CHECK FOR OTHER CALL SITE SUCH AS self. and Self::
            _ => syn::visit_mut::visit_expr_mut(self, i),
        }
    }
}

struct MatchCallSite<'a> {
    caller_fn_name: &'a str,
    callee_finder: &'a mut FindCallee<'a>,
    callee_fn_name: &'a str,
    has_return: bool,
    has_continue: bool,
    has_break: bool,
    enum_str: String,
    added_enum: bool,
}

impl VisitMut for MatchCallSite<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        if self.callee_finder.found {
            return;
        }

        let id = i.sig.ident.to_string();
        match id.contains(self.caller_fn_name) {
            false => (),
            true => {
                self.callee_finder.visit_impl_item_method_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.match_callsite(&mut i.block);
            }
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        if self.callee_finder.found {
            return;
        }

        let id = i.sig.ident.to_string();
        match id.contains(self.caller_fn_name) {
            true => {
                self.callee_finder.visit_item_fn_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                self.match_callsite(&mut i.block);
            }
            false => {}
        }
        syn::visit_mut::visit_item_fn_mut(self, i);
    }


    fn visit_item_mod_mut(&mut self, i: &mut ItemMod) {
        if i.clone()
            .into_token_stream()
            .to_string()
            .contains(self.callee_fn_name)
        {
            match i.content.as_mut() {
                None => {}
                Some((_, items)) => {
                    items.push(syn::parse_str(self.enum_str.as_str()).unwrap());
                    self.added_enum = true;
                }
            }
        }
        syn::visit_mut::visit_item_mod_mut(self, i);
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        if self.callee_finder.found {
            return;
        }

        let id = i.sig.ident.to_string();
        //println!("caller name: {}, at: {}", self.caller_fn_name, &id);
        match id.contains(self.caller_fn_name) {
            false => (),
            true => {
                self.callee_finder.visit_trait_item_method_mut(i);
                if !self.callee_finder.found {
                    return;
                }
                let _ = i
                    .clone()
                    .default
                    .as_mut()
                    .and_then(|block| Some(self.match_callsite(block)));
            }
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }
}

impl MatchCallSite<'_> {
    fn match_callsite(&mut self, block: &mut Block) {
        let mut helper = MatchCallSiteHelper {
            callee_fn_name: self.callee_fn_name,
            has_return: self.has_return,
            has_continue: self.has_continue,
            has_break: self.has_break,
        };
        helper.visit_block_mut(block);
    }
}

#[derive(Debug)]
pub struct NonLocalControlFlowResult {
    pub success: bool,
    pub has_return: bool,
    pub has_continue: bool,
    pub has_break: bool,
    pub num_inputs: usize,
}

pub fn inner_make_controls(
    file_name: &str,
    new_file_name: &str,
    callee_fn_name: &str,
    caller_fn_name: &str,
) -> NonLocalControlFlowResult {
    let mut success = true;
    debug!("debugging controller...");
    let file_content: String = fs::read_to_string(&file_name).unwrap().parse().unwrap();

    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| {
            let s = format!("THERE IS AN ERROR HERE NOT PARSED: {:?}", e);
            println!("errored: {}", &s);
            s
        })
        .unwrap();

    let mut caller_rety = ReturnType::Default;
    let mut caller_visitor = CallerVisitor {
        found: false,
        caller_fn_name,
        callee_finder: &mut FindCallee {
            found: false,
            callee_fn_name,
        },
        callee_fn_name,
        callee_in_loop: false,
        caller_rety: &mut caller_rety,
    };
    caller_visitor.visit_file_mut(&mut file);
    if !caller_visitor.found {
        debug!("did not find caller");
        return NonLocalControlFlowResult {
            success: false,
            has_return: false,
            has_continue: false,
            has_break: false,
            num_inputs: 0,
        };
    }

    let mut callee_visitor = CalleeCheckNCF {
        found: false,
        callee_fn_name,
        caller_rety: caller_visitor.caller_rety.clone(),
        within_caller_loop: caller_visitor.callee_in_loop,
        has_break: false,
        has_continue: false,
        has_return: false,
        num_inputs: 0,
    };
    callee_visitor.visit_file_mut(&mut file);

    if !callee_visitor.found {
        debug!("did not find callee");
        return NonLocalControlFlowResult {
            success: false,
            has_return: false,
            has_continue: false,
            has_break: false,
            num_inputs: 0,
        };
    }

    debug!("callee_visitor: {:?}", callee_visitor);
    if callee_visitor.has_return || callee_visitor.has_continue || callee_visitor.has_break {
        let caller_rety = match caller_visitor.caller_rety {
            ReturnType::Default => Type::Verbatim(quote! {()}),
            ReturnType::Type(_, t) => t.as_ref().clone(),
        };
        let mut already_did_return = false;

        if callee_visitor.has_return {
            debug!("has return!");
            let mut make_ret = MakeReturn {
                callee_fn_name,
                caller_rety: &caller_rety,
            };
            make_ret.visit_file_mut(&mut file);

            let mut make_caller_ret = MakeCallerReturn { callee_fn_name };
            make_caller_ret.visit_file_mut(&mut file);
            already_did_return = true;
        }

        if callee_visitor.has_break || callee_visitor.has_continue {
            debug!("has break {} or cont {}",callee_visitor.has_break, callee_visitor.has_continue);
            let mut make_brk_and_cont = MakeBrkAndCont {
                callee_fn_name,
                success,
                already_did_return,
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

        let mut caller_matcher = MatchCallSite {
            caller_fn_name,
            callee_finder: &mut FindCallee {
                found: false,
                callee_fn_name,
            },
            callee_fn_name,
            has_return: callee_visitor.has_return,
            has_continue: callee_visitor.has_continue,
            has_break: callee_visitor.has_break,
            enum_str: enum_str.clone(),
            added_enum: false,
        };
        caller_matcher.visit_file_mut(&mut file);

        if !caller_matcher.added_enum {
            file.items.push(syn::parse_str(enum_str.as_str()).unwrap());
        }
    }
    let file = file.into_token_stream().to_string();
    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
    NonLocalControlFlowResult {
        success,
        has_return: callee_visitor.has_return,
        has_continue: callee_visitor.has_continue,
        has_break: callee_visitor.has_break,
        num_inputs: callee_visitor.num_inputs,
    }
}

pub fn make_controls(
    file_name: &str,
    new_file_name: &str,
    callee_fn_name: &str,
    caller_fn_name: &str,
) -> bool {
    let res = inner_make_controls(file_name, new_file_name, callee_fn_name, caller_fn_name);
    debug!("result: {:?}", res);
    res.success
}
