use std::collections::HashMap;
use std::ops::Deref;
use itertools::Itertools;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char},
    sequence::{self, delimited},
    IResult,
};
use proc_macro2::{Ident, Span};
use quote::ToTokens;


use syn::{Expr, ExprAssign, FnArg, Stmt, Type, visit::Visit, visit_mut::VisitMut};
use utils::{annotation::Annotations, typ::RustType};
use utils::{labelling::Label, wrappers::IndexWrapper};
use utils::annotation::Annotated;

/// Aliasing Constraints
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AliasConstraints {
    Ref(Label),
    Alias(Label, Label),
    Assign(Label, Label),
}

impl std::fmt::Display for AliasConstraints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AliasConstraints::Ref(r) => write!(f, "ref({})", r),
            AliasConstraints::Alias(l, r) => write!(f, "alias({}, {})", l, r),
            AliasConstraints::Assign(l, r) => write!(f, "assign({}, {})", l, r),
        }
    }
}
impl crate::LocalConstraint for AliasConstraints {
    const CHR_RULES: &'static str = include_str!("constraint_rules/alias_constraint_rules.pl");
    fn parse(s: &str) -> nom::IResult<&str, Self> {
        use utils::parser::{label, rust_type, ws};
        fn ref_(s: &str) -> IResult<&str, AliasConstraints> {
            let (s, _) = tag("ref")(s)?;
            let (s, l1) = delimited(char('('), label, char(')'))(s)?;
            Ok((s, AliasConstraints::Ref(l1)))
        }

        fn alias(s: &str) -> IResult<&str, AliasConstraints> {
            let (s, _) = tag("alias")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, AliasConstraints::Alias(l1, l2)))
        }

        fn assign(s: &str) -> IResult<&str, AliasConstraints> {
            let (s, _) = tag("assign")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, AliasConstraints::Assign(l1, l2)))
        }

        alt((
            ref_,
            alias,
            assign,
        ))(s)
    }

    fn collect<'a>((map, fun): &utils::annotation::Annotated<'a, &'a syn::ItemFn>) -> Vec<Self> {
        use utils::labelling::ASTKey;

        struct Traverse<'a> {
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        fn lookup_ast<'a>(ast: &Annotations<'a>, ident: &dyn ASTKey) -> Option<Label> {
            ast.get(&ident).map(|v| *v)
        }
        fn add_constraint(constraints : &mut Vec<AliasConstraints>, constraint: AliasConstraints) {
            constraints.push(constraint)
        }

        struct IdentHelper<'a> {
            lhs: &'a Label,
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        impl VisitMut for  IdentHelper<'_> {
            fn visit_ident_mut(&mut self, i: &mut Ident) {
                println!("in ident: {}", i.clone().to_string());
                let rhs = lookup_ast(self.ast, i).unwrap();
                add_constraint(self.constraints, AliasConstraints::Assign(self.lhs.clone(), rhs));
                syn::visit_mut::visit_ident_mut(self, i)
            }
        }

        struct ExprHelper<'a>{
            lhs: &'a Label,
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        impl VisitMut for ExprHelper<'_> {
            fn visit_expr_mut(&mut self, i: &mut Expr) {
                let mut id_helper = IdentHelper { lhs: self.lhs, ast: self.ast, constraints: self.constraints };
                id_helper.visit_expr_mut(i);
                match &i {
                    Expr::Reference(_) => add_constraint(self.constraints, AliasConstraints::Ref(*self.lhs)),
                    _ => (),
                }
                syn::visit_mut::visit_expr_mut(self, i)
            }
        }

        struct StmtHelper<'a> {
            lhs: &'a Label,
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        impl VisitMut for StmtHelper<'_> {
            fn visit_stmt_mut(&mut self, i: &mut Stmt) {
                match i {
                    Stmt::Expr(e) => {
                        let mut ident = Ident::new("__IDENT__", Span::call_site());
                        let mut rhs_helper = LHSHelper { ident: &mut ident };
                        rhs_helper.visit_expr_mut(e);
                        if ident.to_string() != "__IDENT__" {
                            let rhs = lookup_ast(self.ast, &ident).unwrap();
                            add_constraint(self.constraints, AliasConstraints::Assign(*self.lhs, rhs));
                        } else {
                            let rhs = lookup_ast(self.ast, e).unwrap();
                            add_constraint(self.constraints, AliasConstraints::Assign(*self.lhs, rhs));
                        }

                        match e {
                            Expr::MethodCall(_) => {} // can ret ref but no idea the ret ty
                            Expr::Call(_) => {}

                            Expr::Block(b) => {
                                match b.block.stmts.last_mut() {
                                    None => {}
                                    Some(s) => self.visit_stmt_mut(s),
                                }
                            }
                            Expr::Box(b) =>  {
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                expr_helper.visit_expr_mut(b.expr.as_mut())
                            },
                            Expr::Cast(c) => {
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                match c.ty.as_ref() {
                                    Type::Reference(_) => expr_helper.visit_expr_mut(c.expr.as_mut()),
                                    _ => (),
                                }
                            }
                            Expr::If(i) => {
                                match i.then_branch.stmts.last_mut() {
                                    None => {}
                                    Some(s) => self.visit_stmt_mut(s),
                                }
                                match &mut i.else_branch {
                                    None => {}
                                    Some((_,s)) => {
                                        self.visit_stmt_mut(&mut Stmt::Expr(*s.clone().clone()))
                                    },
                                }

                            }

                            Expr::Match(m) => {
                                m.arms.iter_mut().for_each(|arm| {
                                    self.visit_stmt_mut(&mut Stmt::Expr(arm.body.as_mut().clone()))
                                })
                            }
                            Expr::Paren(e) => self.visit_stmt_mut(&mut Stmt::Expr(*e.expr.clone())),

                            Expr::Reference(r) => {
                                add_constraint(self.constraints, AliasConstraints::Ref(*self.lhs));
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                expr_helper.visit_expr_mut(r.expr.as_mut())
                            },
                            _=> syn::visit_mut::visit_stmt_mut(self, i),
                        }
                    }
                    _ => syn::visit_mut::visit_stmt_mut(self, i),
                }
            }
            
        }

        struct LHSHelper<'a> {
            ident: &'a mut Ident,
        }

        impl VisitMut for LHSHelper<'_> {
            fn visit_ident_mut(&mut self, i: &mut Ident) {
                *self.ident = i.clone()
            }
        }

        impl VisitMut for Traverse<'_> {
            fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
                self.visit_block_mut(f.block.as_mut());
                for arg in &f.sig.inputs {
                    match arg {
                        FnArg::Typed(ty) => {
                            match ty.ty.as_ref() {
                                Type::Reference(_) => {
                                    let mut ident = Ident::new("IDENT", Span::call_site());
                                    let mut ident_helper = LHSHelper {
                                        ident: &mut ident,
                                    };
                                    ident_helper.visit_pat_mut(ty.pat.clone().as_mut());
                                    let label = lookup_ast(self.ast, &ident).unwrap();
                                    add_constraint(self.constraints, AliasConstraints::Ref(label))
                                }
                                _ => (),
                            }
                        }
                        FnArg::Receiver(_) => (),
                    }
                }
            }

            fn visit_expr_assign_mut(&mut self, i: &mut ExprAssign) {
                let mut ident = Ident::new("IDENT", Span::call_site());
                let mut ident_helper = LHSHelper {ident: &mut ident};
                ident_helper.visit_expr_mut(i.left.as_mut());
                let label = lookup_ast(self.ast, &ident).unwrap();
                let lhs = &label;
                let mut expr_helper = StmtHelper { lhs, ast: self.ast, constraints: self.constraints };
                expr_helper.visit_stmt_mut(&mut Stmt::Expr(*i.right.clone()))
            }

            fn visit_local_mut(&mut self, i: &mut syn::Local) {
                let pat = &i.pat;
                println!("local: {}", i.clone().into_token_stream().to_string());
                match &*pat {
                    // Case of the form `let lhs : T = rhs`
                    syn::Pat::Type(syn::PatType {
                                       pat: box syn::Pat::Ident(p),
                                       ty: box ty,
                                       ..
                                   }) => {
                        match ty {
                            Type::Reference(_) => {
                                let ident = &p.ident;
                                let label = lookup_ast(self.ast, ident).unwrap();
                                let lhs = &label;
                                add_constraint(self.constraints, AliasConstraints::Ref(label));
                                match i.init.clone() {
                                    None => (),
                                    Some((_, mut init)) => {
                                        let mut expr_helper = StmtHelper { lhs, ast: self.ast, constraints: self.constraints };
                                        expr_helper.visit_stmt_mut(&mut Stmt::Expr(*init.clone()))
                                    }
                                }
                            }
                            _ => {}
                        }
                        syn::visit_mut::visit_local_mut(self, i);
                    }
                    syn::Pat::Ident(syn::PatIdent { ident, .. }) => {
                        match i.init.clone() {
                            None => (),
                            Some((_, init)) => {
                                let label = lookup_ast(self.ast, ident).unwrap();
                                let lhs = &label;
                                let mut expr_helper = StmtHelper { lhs, ast: self.ast, constraints: self.constraints };
                                expr_helper.visit_stmt_mut(&mut Stmt::Expr(*init.clone()))
                            }
                        }
                    }
                    _ => syn::visit_mut::visit_local_mut(self, i),
                }
            }
        }

        let mut constraints = vec![];
        let mut collector = Traverse { ast: map, constraints: &mut constraints };
        collector.visit_item_fn_mut(&mut fun.clone().clone());
        constraints.into_iter().unique().collect()
    }
}

/// Array Constraints
#[derive(Clone, Debug)]
pub enum ArrayConstraint {
    /// Eq(A,B)    ==> A = B
    Eq(Label, Label),
    /// Deref(A,B) ==> Deref of A returns B
    Deref(Label, Label),
    /// Compat(A,T) ==> A has type equiv to T
    Compat(Label, RustType),
    /// Offset(A,R) ==> offseting into A with usize returns R
    Offset(Label, Label),
    /// Index(A,R) => Indexing into A with usize returns R
    ShouldIndex(Label, Label),
    /// Index(A,R) => Indexing into A with usize returns R
    Index(Label, Label),
    /// Ref(A,B)      => A is a reference to B
    Ref(Label, Label),
    /// Vec(A)      => A is a vector
    Vec(Label),
    /// Malloc(A)    => A is a malloc
    Malloc(Label),
}

impl std::fmt::Display for ArrayConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayConstraint::Eq(l, r) => write!(f, "{} = {}", l, r),
            ArrayConstraint::Deref(ptr, res) => write!(f, "deref({},{})", ptr, res),
            ArrayConstraint::Compat(lab, ty) => write!(f, "compat({},{})", lab, ty),
            ArrayConstraint::Offset(arr, res) => write!(f, "offset({},{})", arr, res),
            ArrayConstraint::ShouldIndex(arr, res) => write!(f, "shouldindex({},{})", arr, res),
            ArrayConstraint::Index(arr, res) => write!(f, "index({},{})", arr, res),
            ArrayConstraint::Ref(a, b) => write!(f, "ref({},{})", a, b),
            ArrayConstraint::Vec(a) => write!(f, "vec({})", a),
            ArrayConstraint::Malloc(a) => write!(f, "malloc({})", a),
        }
    }
}

impl crate::LocalConstraint for ArrayConstraint {
    const CHR_RULES: &'static str = include_str!("constraint_rules/arr_constraint_rules.pl");

    fn parse(s: &str) -> nom::IResult<&str, Self> {
        use utils::parser::{label, rust_type, ws};
        fn eq(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, (l1, l2)) = sequence::separated_pair(label, ws(char('=')), label)(s)?;
            Ok((s, ArrayConstraint::Eq(l1, l2)))
        }
        fn vec(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("vec")(s)?;
            let (s, l1) = delimited(char('('), label, char(')'))(s)?;
            Ok((s, ArrayConstraint::Vec(l1)))
        }
        fn malloc(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("malloc")(s)?;
            let (s, l1) = delimited(char('('), label, char(')'))(s)?;
            Ok((s, ArrayConstraint::Malloc(l1)))
        }
        fn reference(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("ref")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Ref(l1, l2)))
        }
        fn deref(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("deref")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Deref(l1, l2)))
        }
        fn compat(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("compat")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), rust_type),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Compat(l1, l2)))
        }
        fn offset(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("offset")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Offset(l1, l2)))
        }
        fn index(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("index")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Index(l1, l2)))
        }
        fn shouldindex(s: &str) -> IResult<&str, ArrayConstraint> {
            let (s, _) = tag("shouldindex")(s)?;
            let (s, (l1, l2)) = delimited(
                char('('),
                sequence::separated_pair(label, ws(char(',')), label),
                char(')'),
            )(s)?;
            Ok((s, ArrayConstraint::Index(l1, l2)))
        }
        alt((
            deref,
            compat,
            offset,
            index,
            shouldindex,
            eq,
            vec,
            reference,
            malloc,
        ))(s)
    }

    fn collect<'a>((map, fun): &utils::annotation::Annotated<'a, &'a syn::ItemFn>) -> Vec<Self> {
        use utils::labelling::ASTKey;

        struct Collector<'a, 'b>(&'b Annotations<'a>, Vec<ArrayConstraint>);
        impl<'a, 'b> Collector<'a, 'b> {
            fn lookup_ast<'c>(&self, ident: &'c dyn ASTKey) -> Option<Label> {
                self.0.get(&ident).map(|v| *v)
            }
            fn add_constraint<'c>(&mut self, constraint: ArrayConstraint) {
                self.1.push(constraint)
            }
        }

        impl<'a, 'b, 'ast: 'a> syn::visit::Visit<'ast> for Collector<'a, 'b> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                self.visit_block(&f.block);
            }

            fn visit_local(&mut self, i: &'ast syn::Local) {
                let pat = &i.pat;
                match &*pat {
                    // Case of the form `let lhs : T = rhs`
                    syn::Pat::Type(syn::PatType {
                        pat: box syn::Pat::Ident(p),
                        ty: box ty,
                        ..
                    }) => {
                        let ident = &p.ident;
                        // bind LHS identifier with new label
                        let label = self.lookup_ast(ident).unwrap();
                        if let syn::Type::Path(_e) = ty {
                            // if LHS has type signature, add compat constraint
                            self.add_constraint(ArrayConstraint::Compat(label, ty.clone().into()));
                        }
                        // check the RHS of the let expr
                        // if None (so, an uninitialized variable like "let i;") then we ignore
                        // otherwise, we want to have a constraint stating that LHS = RHS
                        match &i.init {
                            None => (),
                            Some((_, box expr)) => {
                                let expr_label = self.lookup_ast(expr).unwrap();
                                self.add_constraint(ArrayConstraint::Eq(label, expr_label))
                            }
                        }
                        syn::visit::visit_local(self, i);
                    }
                    // Case of the form `let lhs = rhs`
                    syn::Pat::Ident(syn::PatIdent {
                        by_ref,
                        mutability,
                        ident,
                        ..
                    }) => {
                        let label = self.lookup_ast(ident).unwrap();
                        match &i.init {
                            None => (),
                            Some((_, box expr)) => {
                                self.visit_expr(expr);
                                let expr_label = self.lookup_ast(expr).unwrap();
                                match (by_ref, mutability) {
                                    (Some(_r), Some(_m)) => {
                                        self.add_constraint(ArrayConstraint::Ref(
                                            label, expr_label,
                                        ));
                                    }
                                    (Some(_r), None) => {
                                        self.add_constraint(ArrayConstraint::Ref(
                                            label, expr_label,
                                        ));
                                    }
                                    (None, Some(_m)) => (),
                                    (None, None) => (),
                                }
                                self.add_constraint(ArrayConstraint::Eq(label, expr_label))
                            }
                        }
                    }
                    _ => {
                        panic!("use of unsupported ast construct\n{:#?}", i)
                    }
                }
            }

            fn visit_expr(&mut self, i: &'ast syn::Expr) {
                use syn::{
                    Expr, ExprAssign, ExprAssignOp, ExprBinary, ExprMethodCall, ExprParen,
                    ExprPath, ExprUnary, Path, UnOp,
                };
                syn::visit::visit_expr(self, i);
                let label = self.lookup_ast(i).unwrap();
                match i {
                    Expr::Path(ExprPath {
                        path: Path { segments, .. },
                        ..
                    }) => {
                        if segments.len() == 1 && segments[0].arguments.is_empty() {
                            let elt = &segments[0];
                            let ident = &elt.ident;
                            let old_label = self.lookup_ast(ident);
                            match old_label {
                                Some(old_label) => {
                                    self.add_constraint(ArrayConstraint::Eq(old_label, label))
                                }
                                None => {
                                    // assignment is actually a let binding, add a new binding
                                    ()
                                }
                            }
                        } else {
                            // represents either a reference across files?
                            // or a constant ::std::mem::size_of::<_>()...
                            // either case, add no constraints
                            ()
                        }
                    }
                    Expr::Assign(ExprAssign {
                        left: box left,
                        right: box right,
                        ..
                    }) => {
                        let left_label = self.lookup_ast(left).unwrap();
                        let right_label = self.lookup_ast(right).unwrap();
                        self.add_constraint(ArrayConstraint::Eq(left_label, right_label));
                    }
                    Expr::Unary(ExprUnary {
                        attrs: _,
                        op,
                        box expr,
                    }) => {
                        let body_label = self.lookup_ast(expr).unwrap();
                        match *op {
                            UnOp::Deref(_) => {
                                self.add_constraint(ArrayConstraint::Deref(body_label, label))
                            }
                            UnOp::Not(_) => {
                                self.add_constraint(ArrayConstraint::Eq(body_label, label))
                            }
                            UnOp::Neg(_) => {
                                self.add_constraint(ArrayConstraint::Eq(body_label, label))
                            }
                        }
                    }
                    Expr::Index(syn::ExprIndex { expr, index, .. }) => {
                        self.visit_expr(expr);
                        syn::visit::visit_expr(self, index);
                    }
                    Expr::Cast(syn::ExprCast {
                        expr: box expr,
                        ty: box ty,
                        ..
                    }) => {
                        let r_ty: RustType = ty.clone().into();
                        match expr {
                            Expr::Path(syn::ExprPath {
                                path: syn::Path { segments, .. },
                                ..
                            }) => {
                                let ident = &segments[0].ident;
                                match r_ty {
                                    RustType::CVoid => (),
                                    RustType::Pointer(box RustType::CVoid) => (),
                                    _ => match self.lookup_ast(ident) {
                                        None => {
                                            let expr_label = self.lookup_ast(expr).unwrap();
                                            self.add_constraint(ArrayConstraint::Compat(
                                                expr_label,
                                                ty.clone().into(),
                                            ));
                                            self.add_constraint(ArrayConstraint::Eq(
                                                label, expr_label,
                                            ));
                                        }
                                        Some(_lbl) => {
                                            self.add_constraint(ArrayConstraint::Compat(
                                                label,
                                                ty.clone().into(),
                                            ));
                                        }
                                    },
                                }
                            }
                            Expr::Call(syn::ExprCall {
                                func:
                                    box Expr::Path(syn::ExprPath {
                                        path: syn::Path { segments, .. },
                                        ..
                                    }),
                                args: _,
                                ..
                            }) => {
                                let ident = &segments[0].ident;
                                // we want to ignore Casts of Mallocs, since this wrongfully
                                // gives us a constraint for `let arr = malloc as *i32` that arr is *i32
                                // when in fact we're using it as an array later on
                                // we want to check other calls, e.g., `f(arr as *i32)` since these
                                // point to actual uses of arr as a pointer
                                if ident.to_string() == "malloc" {
                                    let expr_label = self.lookup_ast(expr).unwrap();
                                    self.add_constraint(ArrayConstraint::Eq(label, expr_label));
                                } else {
                                    let expr_label = self.lookup_ast(expr).unwrap();
                                    self.add_constraint(ArrayConstraint::Compat(
                                        expr_label,
                                        ty.clone().into(),
                                    ));
                                    self.add_constraint(ArrayConstraint::Eq(label, expr_label));
                                }
                            }
                            _ => match r_ty {
                                RustType::CVoid => (),
                                RustType::Pointer(box RustType::CVoid) => (),
                                _ => {
                                    let expr_label = self.lookup_ast(expr).unwrap();
                                    self.add_constraint(ArrayConstraint::Compat(
                                        expr_label,
                                        ty.clone().into(),
                                    ));
                                    self.add_constraint(ArrayConstraint::Eq(label, expr_label));
                                }
                            },
                        }
                    }
                    Expr::MethodCall(ExprMethodCall {
                        receiver: box receiver,
                        method,
                        args,
                        ..
                    }) => {
                        // offset method call
                        if method.to_string().as_str() == "offset" && args.len() == 1 {
                            let _offset_ind = &args[0];
                            let expr_label = self.lookup_ast(receiver).unwrap();
                            self.add_constraint(ArrayConstraint::Offset(expr_label, label));
                        } else {
                            () // don't handle other method calls yet
                        }
                    }
                    Expr::Binary(ExprBinary {
                        op: _,
                        left: box left_expr,
                        right: box right_expr,
                        ..
                    }) => {
                        let left_label = self.lookup_ast(left_expr).unwrap();
                        let right_label = self.lookup_ast(right_expr).unwrap();
                        self.add_constraint(ArrayConstraint::Eq(left_label, right_label))
                    }
                    Expr::AssignOp(ExprAssignOp {
                        op: _,
                        left: box left,
                        right: box right,
                        ..
                    }) => {
                        let left_label = self.lookup_ast(left).unwrap();
                        let right_label = self.lookup_ast(right).unwrap();
                        self.add_constraint(ArrayConstraint::Eq(left_label, right_label))
                    }
                    Expr::Paren(ExprParen { expr: box expr, .. }) => {
                        let inner_label = self.lookup_ast(expr).unwrap();
                        self.add_constraint(ArrayConstraint::Eq(inner_label, label))
                    }

                    Expr::Return(_) => (),
                    Expr::Reference(syn::ExprReference {
                        mutability: _,
                        box expr,
                        ..
                    }) => {
                        let ident_label = self.lookup_ast(expr).unwrap();
                        //e.g., (&mut x: L1):L2 means that L2 is a reference to L1, and that L2 is mutable
                        self.add_constraint(ArrayConstraint::Ref(label, ident_label));
                    }
                    expr @ Expr::Call(_) if IndexWrapper::is_index_wrapper(expr) => {
                        let inner = IndexWrapper::from(expr.clone());
                        let base_expr_label = self.lookup_ast(inner.base_expr()).unwrap();

                        let labels = {
                            let extract_label = |expr: &syn::Expr| self.lookup_ast(expr).unwrap();
                            IndexWrapper::fold_calls(extract_label, expr)
                        };
                        // we expect 1 more label than the level
                        // of indirection because of the extra
                        // IndexWrapperFinal
                        // i.e 2 => IndexWrapperFinal(IndexWrapper(IndexWrapperBase(_)))
                        assert_eq!(labels.len(), inner.indirection() + 1);

                        let mut labels = labels.into_iter();
                        let mut current_label = labels.next().unwrap();
                        self.add_constraint(ArrayConstraint::Eq(current_label, base_expr_label));

                        while let Some(next_label) = labels.next() {
                            self.add_constraint(ArrayConstraint::Index(current_label, next_label));
                            current_label = next_label;
                        }

                        self.add_constraint(ArrayConstraint::Compat(
                            current_label,
                            inner.base_ty().clone(),
                        ))
                    }
                    Expr::Call(syn::ExprCall {
                        args,
                        func:
                            box Expr::Path(syn::ExprPath {
                                path: syn::Path { segments, .. },
                                ..
                            }),
                        ..
                    }) => {
                        let fn_name = &segments[0].ident.to_string();
                        // if call is to malloc, add constraint expressing that.
                        if fn_name == "malloc" {
                            self.add_constraint(ArrayConstraint::Malloc(label));
                        }

                        for expr in args.iter() {
                            //assume that exprs, if they are arguments to a function, are mutable
                            if fn_name != "free" {
                                self.visit_expr(expr);
                            }
                            //let expr_label = self.lookup_ast(expr).unwrap();
                            //self.add_constraint(ArrayConstraint::Mut(expr_label));
                        }
                    }
                    Expr::Unsafe(syn::ExprUnsafe { .. }) => (), //not done
                    _ => syn::visit::visit_expr(self, i),
                }
            }
        }

        let mut collector = Collector(map, Default::default());

        collector.visit_item_fn(fun);

        collector.1
    }
}

/// Mutability Constraints
#[derive(Clone, Debug)]
pub enum MutabilityConstraint {
    /// Mut(A)       => A is used mutably
    Mut(Label),
}

impl std::fmt::Display for MutabilityConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutabilityConstraint::Mut(arr) => write!(f, "mut({})", arr),
        }
    }
}

impl crate::LocalConstraint for MutabilityConstraint {
    const CHR_RULES: &'static str = include_str!("constraint_rules/arr_constraint_rules.pl");

    fn parse(s: &str) -> nom::IResult<&str, Self> {
        use utils::parser::{label};
        fn muta(s: &str) -> IResult<&str, MutabilityConstraint> {
            let (s, _) = tag("mut")(s)?;
            let (s, l1) = delimited(char('('), label, char(')'))(s)?;
            Ok((s, MutabilityConstraint::Mut(l1)))
        }
        muta(s)
    }

    fn collect<'a>((map, fun): &utils::annotation::Annotated<'a, &'a syn::ItemFn>) -> Vec<Self> {
        use utils::labelling::ASTKey;

        struct Collector<'a, 'b>(
            &'b HashMap<&'a dyn ASTKey, Label>,
            Vec<MutabilityConstraint>,
        );
        impl<'a, 'b> Collector<'a, 'b> {
            fn lookup_ast<'c>(&self, ident: &'c dyn ASTKey) -> Option<Label> {
                self.0.get(&ident).map(|v| *v)
            }
            fn add_constraint<'c>(&mut self, constraint: MutabilityConstraint) {
                self.1.push(constraint)
            }
        }

        impl<'a, 'b, 'ast: 'a> syn::visit::Visit<'ast> for Collector<'a, 'b> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                for arg in f.sig.inputs.iter() {
                    match arg {
                        // item functions are standalone functions - we should never see a self
                        syn::FnArg::Receiver(_) => unreachable!(),
                        syn::FnArg::Typed(syn::PatType {
                            pat:
                                box syn::Pat::Ident(syn::PatIdent {
                                    ident,
                                    subpat: None,
                                    mutability: is_mutable,
                                    ..
                                }),
                            ..
                        }) => {
                            let ident_label = self.lookup_ast(ident).unwrap();

                            match is_mutable {
                                Some(_) => {
                                    self.add_constraint(MutabilityConstraint::Mut(ident_label))
                                }
                                None => (),
                            }
                        }
                        _ => panic!("use of unsupported syntactic form {:?}", f),
                    }
                }
                self.visit_block(&f.block);
            }

            fn visit_local(&mut self, i: &'ast syn::Local) {
                match &i.pat {
                    // Case of the form `let lhs : T = rhs`
                    syn::Pat::Type(syn::PatType {
                        pat: box syn::Pat::Ident(p),
                        ty: box _ty,
                        ..
                    }) => {
                        let ident = &p.ident;
                        // bind LHS identifier with new label
                        let _label = self.lookup_ast(ident).unwrap();
                        // check the RHS of the let expr
                        // if None (so, an uninitialized variable like "let i;") then we ignore
                        // otherwise, we want to have a constraint stating that LHS = RHS
                        let rhs = &i.init;
                        match rhs {
                            None => (),
                            Some((_, box expr)) => {
                                self.visit_expr(&expr);
                            }
                        }
                        syn::visit::visit_local(self, i);
                    }
                    // Case of the form `let lhs = rhs`
                    syn::Pat::Ident(syn::PatIdent {
                        by_ref,
                        mutability,
                        ident,
                        ..
                    }) => {
                        let label = self.lookup_ast(ident).unwrap();
                        match &i.init {
                            None => (),
                            Some((_, box expr)) => {
                                self.visit_expr(expr);
                                let _expr_label = self.lookup_ast(expr).unwrap();
                                match (by_ref, mutability) {
                                    (Some(_r), Some(_m)) => {
                                        self.add_constraint(MutabilityConstraint::Mut(label));
                                    }
                                    (Some(_r), None) => {}
                                    (None, Some(_m)) => {
                                        self.add_constraint(MutabilityConstraint::Mut(label));
                                    }
                                    (None, None) => (),
                                }
                            }
                        }
                    }
                    e => {
                        panic!("use of unsupported ast construct {:#?}", e)
                    }
                }
            }

            fn visit_expr(&mut self, i: &'ast syn::Expr) {
                use syn::{Expr, ExprAssign};
                let label = self.lookup_ast(i).unwrap();
                syn::visit::visit_expr(self, i);

                match i {
                    Expr::Assign(ExprAssign {
                        left: box left,
                        right: box right,
                        ..
                    }) => {
                        let left_label = self.lookup_ast(left).unwrap();
                        let _right_label = self.lookup_ast(right).unwrap();
                        self.add_constraint(MutabilityConstraint::Mut(left_label));
                    }
                    Expr::Reference(syn::ExprReference {
                        mutability,
                        box expr,
                        ..
                    }) => {
                        let _ident_label = self.lookup_ast(expr).unwrap();
                        //e.g., (&mut x: L1):L2 means that L2 is a reference to L1, and that L2 is mutable
                        match mutability {
                            None => (),
                            Some(_) => {
                                self.add_constraint(MutabilityConstraint::Mut(label));
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        let mut collector = Collector(map, Default::default());

        collector.visit_item_fn(fun);

        collector.1
    }
}
