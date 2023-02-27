use proc_macro2::Span;
use quote::ToTokens;
use std::borrow::BorrowMut;
use std::fs;
use syn::{visit_mut::VisitMut, FnArg, GenericArgument, Lifetime, LifetimeDef, PathArguments, ReturnType, Type, TypeParamBound, AngleBracketedGenericArguments};

use crate::common::{
    elide_lifetimes_annotations, repair_bounds_help, repair_iteration, repair_iteration_project,
    RepairSystem, RustcError,
};
use crate::repair_lifetime_simple;
use utils::{compile_file, compile_project, format_source};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_loosest_bounds_first_repairer"
    }

    fn repair_project(&self, src_path: &str, manifest_path: &str, fn_name: &str) -> bool {
        annotate_loose_named_lifetime(src_path, fn_name);
        // println!("annotated: {}", fs::read_to_string(&src_path).unwrap());
        let mut compile_cmd = compile_project(manifest_path, &vec![]);
        let process_errors =
            |ce: &RustcError| repair_bounds_help(ce.rendered.as_str(), src_path, fn_name);
        match repair_iteration_project(&mut compile_cmd, src_path, &process_errors, true, Some(50))
        {
            true => {
                elide_lifetimes_annotations(src_path, fn_name);
                true
            }
            false => false,
        }
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> bool {
        repair_lifetime_simple::Repairer {}.repair_file(file_name, new_file_name)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, fn_name: &str) -> bool {
        fs::copy(file_name, &new_file_name).unwrap();
        annotate_loose_named_lifetime(&new_file_name, fn_name);
        // println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args: Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &str| repair_bounds_help(stderr, new_file_name, fn_name);

        match repair_iteration(&mut compile_cmd, &process_errors, true, Some(50)) {
            true => {
                // println!("repaired: {}", fs::read_to_string(&new_file_name).unwrap());
                elide_lifetimes_annotations(new_file_name, fn_name);
                true
            }
            false => false,
        }
    }
}

struct LooseLifetimeAnnotatorTypeHelper {
    lt_num: i32,
}

impl VisitMut for LooseLifetimeAnnotatorTypeHelper {
    fn visit_type_mut(&mut self, i: &mut Type) {
        // println!(
        //     "visiting type: {} {:?}",
        //     i.clone().into_token_stream().to_string(),
        //     i.clone()
        // );
        match i {
            Type::Reference(r) => {
                r.lifetime = Some(Lifetime::new(
                    format!("'lt{}", self.lt_num).as_str(),
                    Span::call_site(),
                ));
                self.lt_num += 1;
                syn::visit_mut::visit_type_mut(self, i);
            }
            Type::TraitObject(t) => {
                //  println!(
                //     "annotating trait obj: {}...",
                //     t.clone().into_token_stream().to_string()
                // );
                t.bounds.iter_mut().for_each(|x| match x {
                    TypeParamBound::Trait(_) => (),
                    TypeParamBound::Lifetime(lt) => {
                        *lt = Lifetime::new(
                            format!("'lt{}", self.lt_num).as_str(),
                            Span::call_site(),
                        );
                        self.lt_num += 1;
                    }
                });
                syn::visit_mut::visit_type_mut(self, i);
            }
            Type::Path(p) => {
                p.path
                    .segments
                    .iter_mut()
                    .for_each(|ps| match ps.arguments.borrow_mut() {
                        PathArguments::AngleBracketed(tf) => tf.args.iter_mut().for_each(|arg| {
                            match arg {
                                GenericArgument::Lifetime(lt) => {
                                    if !lt.clone().ident.to_string().starts_with("lt") {
                                        *lt = Lifetime::new(
                                            format!("'lt{}", self.lt_num).as_str(),
                                            Span::call_site(),
                                        );
                                        self.lt_num += 1;
                                    }
                                }
                                _ => syn::visit_mut::visit_generic_argument_mut(self, arg),
                            };
                        }),
                        ps_arg => syn::visit_mut::visit_path_arguments_mut(self, ps_arg),
                    });
                syn::visit_mut::visit_type_mut(self, i);
            }
            _ => syn::visit_mut::visit_type_mut(self, i),
        }
    }
}

struct LooseLifetimeAnnotatorFnArgHelper {
    lt_num: i32,
}

impl VisitMut for LooseLifetimeAnnotatorFnArgHelper {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(r) => match &mut r.reference {
                None => {}
                Some((_, lt)) => {
                    *lt = Some(Lifetime::new(
                        format!("'lt{}", self.lt_num).as_str(),
                        Span::call_site(),
                    ));
                    self.lt_num += 1;
                }
            },
            FnArg::Typed(t) => {
                let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                    lt_num: self.lt_num,
                };
                type_helper.visit_type_mut(t.ty.as_mut());
                self.lt_num = type_helper.lt_num
            }
        }
    }

    fn visit_angle_bracketed_generic_arguments_mut(&mut self, i: &mut AngleBracketedGenericArguments) {
        i.args.iter_mut().for_each(|arg| {
            match arg {
                GenericArgument::Lifetime(lt) => {
                    *lt = Lifetime::new(
                        format!("'lt{}", self.lt_num).as_str(),
                        Span::call_site(),
                    );
                    self.lt_num += 1;
                }
                gen => {
                    let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                        lt_num: self.lt_num,
                    };
                    type_helper.visit_generic_argument_mut(gen);
                    self.lt_num = type_helper.lt_num
                }
            }
        });
        syn::visit_mut::visit_angle_bracketed_generic_arguments_mut(self, i);
    }
}

struct LooseLifetimeAnnotator<'a> {
    fn_name: &'a str,
    lt_num: i32,
    success: bool,
}

impl VisitMut for LooseLifetimeAnnotator<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => match (&mut i.sig.inputs, &mut i.sig.generics, &mut i.sig.output) {
                (inputs, _, _) if inputs.len() == 0 => self.success = true,
                (inputs, gen, out) => {
                    inputs.iter_mut().for_each(|arg| {
                        let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper {
                            lt_num: self.lt_num,
                        };
                        fn_arg_helper.visit_fn_arg_mut(arg);
                        self.lt_num = fn_arg_helper.lt_num
                    });
                    gen.params.iter_mut().for_each(|param| {
                        let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper {
                            lt_num: self.lt_num,
                        };
                        fn_arg_helper.visit_generic_param_mut(param);
                        self.lt_num = fn_arg_helper.lt_num
                    });
                    match out {
                        ReturnType::Type(_, ty) => {
                            let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                                lt_num: self.lt_num,
                            };
                            type_helper.visit_type_mut(ty.as_mut());
                            self.lt_num = type_helper.lt_num
                        }
                        ReturnType::Default => {}
                    };
                    for lt in 0..self.lt_num {
                        let lifetime =
                            Lifetime::new(format!("'lt{}", lt).as_str(), Span::call_site());
                        gen.params.push(syn::GenericParam::Lifetime(LifetimeDef {
                            attrs: vec![],
                            lifetime,
                            colon_token: None,
                            bounds: Default::default(),
                        }))
                    }
                    self.success = true
                }
            },
        }
    }
}

pub fn annotate_loose_named_lifetime(new_file_name: &str, fn_name: &str) -> bool {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut visit = LooseLifetimeAnnotator {
        fn_name,
        success: false,
        lt_num: 0,
    };
    visit.visit_file_mut(&mut file);
    let file = file.into_token_stream().to_string();
    match visit.success {
        true => {
            fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
            true
        }
        false => false,
    }
}
