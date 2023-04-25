use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;

use syn::visit::Visit;
use syn::{Expr, ExprPath, ItemFn, Path};

use crate::labelling::ASTKey;
use crate::labelling::Label;

/// Annotations of an AST
pub type Annotations<'a> = HashMap<&'a dyn ASTKey, Label>;

/// A pair of an AST and its annotations
pub type Annotated<'a, T> = (Annotations<'a>, T);

pub const LOOKUP_FILE: &str = "/tmp/annotation_rev_lookup";

/// Internal helper struct to annotate an AST
struct ASTAnnotator<'a> {
    annotations: Annotations<'a>,
    next_label: Label,
    env: crate::labelling::ScopedContext<syn::Ident, Label>,
}

impl<'a> ASTAnnotator<'a> {
    pub fn init() -> Self {
        let map = HashMap::new();
        let label = Label::new();
        let context = Default::default();
        fs::write(LOOKUP_FILE, "").unwrap();
        ASTAnnotator {
            annotations: map,
            next_label: label,
            env: context,
        }
    }

    // return annotations
    pub fn annotations(self) -> HashMap<&'a dyn ASTKey, Label> {
        self.annotations
    }

    fn add_binding(&mut self, var: &'a syn::Ident, value: Label) {
        self.env.add_binding(var.clone(), value)
    }

    // check if Ident, if not look up expr in AST map
    pub fn lookup_expr(&mut self, expr: &'a syn::Expr) -> Option<Label> {
        if let Expr::Path(syn::ExprPath {
            path: syn::Path { segments, .. },
            ..
        }) = expr
        {
            let ident = &segments.last().unwrap().ident;
            self.lookup(ident)
        } else {
            self.lookup_ast(expr)
        }
    }

    pub fn lookup_ast<'b>(&self, ident: &'b dyn ASTKey) -> Option<Label> {
        self.annotations.get(&ident).map(|v| *v)
    }

    fn lookup(&mut self, ident: &'a syn::Ident) -> Option<Label> {
        self.env.lookup(ident)
    }

    pub fn open_scope(&mut self) {
        self.env.open_scope()
    }

    pub fn close_scope(&mut self) {
        self.env.close_scope()
    }

    fn new_label(&mut self) -> Label {
        let label = self.next_label;
        self.next_label.incr();
        label
    }
}

impl<'a> syn::visit::Visit<'a> for ASTAnnotator<'a> {
    fn visit_item_fn(&mut self, f: &'a syn::ItemFn) {
        for arg in f.sig.inputs.iter() {
            match arg {
                // item functions are standalone functions - we should never see a self
                syn::FnArg::Receiver(_) => unreachable!(),
                syn::FnArg::Typed(syn::PatType {
                    pat:
                        box syn::Pat::Ident(syn::PatIdent {
                            ident,
                            subpat: None,
                            ..
                        }),
                    ..
                }) => {
                    let value = self.new_label();
                    self.annotations.insert(ident, value);
                    // println!("{} -> {}", value, ident);
                    let mut file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(LOOKUP_FILE)
                        .unwrap();
                    writeln!(file, "{} -> {}", value, ident).unwrap();
                    self.add_binding(ident, value)
                }
                _ => (),
            }
        }
        self.visit_block(&f.block);
    }

    // handle scopes
    fn visit_block(&mut self, i: &'a syn::Block) {
        self.open_scope();
        let res = syn::visit::visit_block(self, i);
        self.close_scope();
        res
    }

    // update local mapping if dealing with a let binding
    fn visit_local(&mut self, i: &'a syn::Local) {
        syn::visit::visit_local(self, i);
        let label = self.new_label();
        match &i.pat {
            // Case of the form `let lhs : T = rhs`
            syn::Pat::Type(syn::PatType {
                pat: box syn::Pat::Ident(p),
                ty: box _ty,
                ..
            }) => {
                let ident = &p.ident;
                // bind LHS identifier with new label
                self.add_binding(ident, label);
                self.annotations.insert(ident, label);
                // println!("{} -> {}", label, ident);
                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(LOOKUP_FILE)
                    .unwrap();
                writeln!(file, "{} -> {}", label, ident).unwrap();
                self.annotations.insert(&i.pat, label);
            }
            // Case of the form `let lhs = rhs`
            syn::Pat::Ident(syn::PatIdent {
                by_ref: _,
                mutability: _,
                ident,
                ..
            }) => {
                self.add_binding(ident, label);
                self.annotations.insert(ident, label);
                // println!("{} -> {}", label, ident);
                let mut file = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(LOOKUP_FILE)
                    .unwrap();
                writeln!(file, "{} -> {}", label, ident).unwrap();
                self.annotations.insert(&i.pat, label);
            }
            _lb => {
                /*panic!(
                    "use of unsupported syntactic form {:#?}",
                    lb.into_token_stream().to_string()
                )*/
                let label = self.new_label();
                self.annotations.insert(i, label);
            }
        }
    }

    fn visit_expr(&mut self, i: &'a Expr) {
        // first visit children
        syn::visit::visit_expr(self, i);
        match i {
            // special case identifiers
            Expr::Path(ExprPath {
                path: Path { segments, .. },
                ..
            }) => {
                // lookup identifier in context
                let elt = &segments[0];
                let ident = &elt.ident;
                let old_label = self.lookup(ident);
                match old_label {
                    Some(label) => {
                        self.annotations.insert(i, label);
                    }
                    None => {
                        // identifier that isn't in the context => refers to a function call
                        // add a label for posterity
                        let label = self.new_label();
                        self.annotations.insert(i, label);
                        self.add_binding(ident, label);
                    }
                }
            }
            _ => {
                // otherwise, some arbitrary expression, add label to it
                let label = self.new_label();
                self.annotations.insert(i, label);
            }
        }
    }
}

/// Annotates a Rust AST
pub fn annotate_ast<'a>(ast: &'a ItemFn) -> Annotated<'a, &'a ItemFn> {
    let mut ast_annotation = ASTAnnotator::init();

    ast_annotation.visit_item_fn(ast);

    (ast_annotation.annotations(), ast)
}
