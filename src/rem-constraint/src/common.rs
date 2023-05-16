use itertools::Itertools;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    sequence::{self, delimited},
    IResult,
};
use proc_macro2::{Ident, Span};

use rem_utils::annotation::Annotations;
use rem_utils::labelling::Label;
use syn::{visit_mut::VisitMut, Expr, ExprAssign, FnArg, Stmt, Type};

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
        use rem_utils::parser::{label, ws};
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

        alt((ref_, alias, assign))(s)
    }

    fn collect<'a>(
        (map, fun): &rem_utils::annotation::Annotated<'a, &'a syn::ItemFn>,
    ) -> Vec<Self> {
        use rem_utils::labelling::ASTKey;

        struct Traverse<'a> {
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        fn lookup_ast<'a>(ast: &Annotations<'a>, ident: &dyn ASTKey) -> Option<Label> {
            ast.get(&ident).map(|v| *v)
        }
        fn add_constraint(constraints: &mut Vec<AliasConstraints>, constraint: AliasConstraints) {
            constraints.push(constraint)
        }

        struct IdentHelper<'a> {
            lhs: &'a Label,
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        impl VisitMut for IdentHelper<'_> {
            fn visit_ident_mut(&mut self, i: &mut Ident) {
                // println!("in ident: {}", i.clone().to_string());
                lookup_ast(self.ast, i).and_then(|rhs| {
                    if *self.lhs.clone().to_string() != rhs.clone().to_string() {
                        add_constraint(
                            self.constraints,
                            AliasConstraints::Assign(self.lhs.clone(), rhs),
                        );
                    }
                    Some(())
                });
                syn::visit_mut::visit_ident_mut(self, i)
            }
        }

        struct ExprHelper<'a> {
            lhs: &'a Label,
            ast: &'a Annotations<'a>,
            constraints: &'a mut Vec<AliasConstraints>,
        }

        impl VisitMut for ExprHelper<'_> {
            fn visit_expr_mut(&mut self, i: &mut Expr) {
                let mut id_helper = IdentHelper {
                    lhs: self.lhs,
                    ast: self.ast,
                    constraints: self.constraints,
                };
                id_helper.visit_expr_mut(i);
                match &i {
                    Expr::Reference(_) => {
                        add_constraint(self.constraints, AliasConstraints::Ref(*self.lhs))
                    }
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
                            lookup_ast(self.ast, &ident).and_then(|rhs| {
                                if *self.lhs.clone().to_string() != rhs.clone().to_string() {
                                    add_constraint(
                                        self.constraints,
                                        AliasConstraints::Assign(*self.lhs, rhs),
                                    );
                                }
                                Some(())
                            });
                        } else {
                            lookup_ast(self.ast, e).and_then(|rhs| {
                                if *self.lhs.clone().to_string() != rhs.clone().to_string() {
                                    add_constraint(
                                        self.constraints,
                                        AliasConstraints::Assign(*self.lhs, rhs),
                                    );
                                }
                                Some(())
                            });
                        }

                        match e {
                            Expr::MethodCall(_) => {} // can ret ref but no idea the ret ty
                            Expr::Call(_) => {}

                            Expr::Block(b) => match b.block.stmts.last_mut() {
                                None => {}
                                Some(s) => self.visit_stmt_mut(s),
                            },
                            Expr::Box(b) => {
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                expr_helper.visit_expr_mut(b.expr.as_mut())
                            }
                            Expr::Cast(c) => {
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                match c.ty.as_ref() {
                                    Type::Reference(_) => {
                                        expr_helper.visit_expr_mut(c.expr.as_mut())
                                    }
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
                                    Some((_, s)) => {
                                        self.visit_stmt_mut(&mut Stmt::Expr(*s.clone().clone()))
                                    }
                                }
                            }

                            Expr::Match(m) => m.arms.iter_mut().for_each(|arm| {
                                self.visit_stmt_mut(&mut Stmt::Expr(arm.body.as_mut().clone()))
                            }),
                            Expr::Paren(e) => self.visit_stmt_mut(&mut Stmt::Expr(*e.expr.clone())),

                            Expr::Reference(r) => {
                                add_constraint(self.constraints, AliasConstraints::Ref(*self.lhs));
                                let mut expr_helper = ExprHelper {
                                    lhs: self.lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                expr_helper.visit_expr_mut(r.expr.as_mut())
                            }
                            _ => syn::visit_mut::visit_stmt_mut(self, i),
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
                        FnArg::Typed(ty) => match ty.ty.as_ref() {
                            Type::Reference(_) => {
                                let mut ident = Ident::new("IDENT", Span::call_site());
                                let mut ident_helper = LHSHelper { ident: &mut ident };
                                ident_helper.visit_pat_mut(ty.pat.clone().as_mut());
                                lookup_ast(self.ast, &ident).and_then(|label| {
                                    add_constraint(self.constraints, AliasConstraints::Ref(label));
                                    Some(())
                                });
                            }
                            _ => (),
                        },
                        FnArg::Receiver(_) => (),
                    }
                }
            }

            fn visit_expr_assign_mut(&mut self, i: &mut ExprAssign) {
                let mut ident = Ident::new("IDENT", Span::call_site());
                let mut ident_helper = LHSHelper { ident: &mut ident };
                ident_helper.visit_expr_mut(i.left.as_mut());
                lookup_ast(self.ast, &ident).and_then(|label| {
                    let lhs = &label;
                    let mut expr_helper = StmtHelper {
                        lhs,
                        ast: self.ast,
                        constraints: self.constraints,
                    };
                    expr_helper.visit_stmt_mut(&mut Stmt::Expr(*i.right.clone()));
                    Some(())
                });
            }

            fn visit_local_mut(&mut self, i: &mut syn::Local) {
                let pat = &i.pat;
                // println!("local: {}", i.clone().into_token_stream().to_string());
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
                                lookup_ast(self.ast, ident).and_then(|label| {
                                    let lhs = &label;
                                    add_constraint(self.constraints, AliasConstraints::Ref(label));
                                    match i.init.clone() {
                                        None => (),
                                        Some((_, init)) => {
                                            let mut expr_helper = StmtHelper {
                                                lhs,
                                                ast: self.ast,
                                                constraints: self.constraints,
                                            };
                                            expr_helper
                                                .visit_stmt_mut(&mut Stmt::Expr(*init.clone()))
                                        }
                                    };
                                    Some(())
                                });
                            }
                            _ => {}
                        }
                        syn::visit_mut::visit_local_mut(self, i);
                    }
                    syn::Pat::Ident(syn::PatIdent { ident, .. }) => match i.init.clone() {
                        None => (),
                        Some((_, init)) => {
                            lookup_ast(self.ast, ident).and_then(|label| {
                                let lhs = &label;
                                let mut expr_helper = StmtHelper {
                                    lhs,
                                    ast: self.ast,
                                    constraints: self.constraints,
                                };
                                expr_helper.visit_stmt_mut(&mut Stmt::Expr(*init.clone()));
                                Some(())
                            });
                        }
                    },
                    _ => syn::visit_mut::visit_local_mut(self, i),
                }
            }
        }

        let mut constraints = vec![];
        let mut collector = Traverse {
            ast: map,
            constraints: &mut constraints,
        };
        collector.visit_item_fn_mut(&mut fun.clone().clone());
        constraints.into_iter().unique().collect()
    }
}
