use log::debug;
use proc_macro2::Span;
use quote::ToTokens;
use std::borrow::BorrowMut;
use std::fs;
use syn::{
    visit_mut::VisitMut, AngleBracketedGenericArguments, FnArg, GenericArgument, ImplItemMethod,
    Lifetime, LifetimeDef, PathArguments, ReturnType, Signature, TraitItemMethod, Type,
    TypeParamBound,
};

use crate::common::{
    callee_renamer, elide_lifetimes_annotations, repair_bounds_help, repair_iteration,
    repair_iteration_project, RepairResult, RepairSystem, RustcError,
};
use crate::repair_lifetime_simple;
use rem_utils::{check_project, compile_file, format_source};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_loosest_bounds_first_repairer"
    }

    fn repair_project(&self, src_path: &str, manifest_path: &str, fn_name: &str) -> RepairResult {
        let annot_res = annotate_loose_named_lifetime(src_path, fn_name);
        if !annot_res.success {
            return RepairResult {
                success: false,
                repair_count: 0,
                has_non_elidible_lifetime: false,
                has_struct_lt: false,
            };
        }
        // println!("annotated: {}", fs::read_to_string(&src_path).unwrap());
        let mut compile_cmd = check_project(manifest_path, &vec![]);
        let process_errors =
            |ce: &RustcError| repair_bounds_help(ce.rendered.as_str(), src_path, fn_name);
        match repair_iteration_project(&mut compile_cmd, src_path, &process_errors, true, Some(50))
        {
            RepairResult {
                success: true,
                repair_count,
                ..
            } => {
                debug!("pre elision: {}", fs::read_to_string(&src_path).unwrap());
                let elide_res = elide_lifetimes_annotations(src_path, fn_name);
                callee_renamer(src_path, fn_name);
                RepairResult {
                    success: true,
                    repair_count,
                    has_non_elidible_lifetime: elide_res.annotations_left,
                    has_struct_lt: elide_res.has_struct_lt || annot_res.has_struct_lt,
                }
            }
            result => result,
        }
    }

    fn repair_file(&self, file_name: &str, new_file_name: &str) -> RepairResult {
        repair_lifetime_simple::Repairer {}.repair_file(file_name, new_file_name)
    }

    fn repair_function(&self, file_name: &str, new_file_name: &str, fn_name: &str) -> RepairResult {
        fs::copy(file_name, &new_file_name).unwrap();
        annotate_loose_named_lifetime(&new_file_name, fn_name);
        // println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args: Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &str| repair_bounds_help(stderr, new_file_name, fn_name);

        match repair_iteration(&mut compile_cmd, &process_errors, true, Some(50)) {
            RepairResult {
                success: true,
                repair_count,
                ..
            } => {
                // println!("repaired: {}", fs::read_to_string(&new_file_name).unwrap());
                let elide_res = elide_lifetimes_annotations(new_file_name, fn_name);
                RepairResult {
                    success: true,
                    repair_count,
                    has_non_elidible_lifetime: elide_res.annotations_left,
                    has_struct_lt: elide_res.has_struct_lt,
                }
            }
            result => result,
        }
    }
}

struct LooseLifetimeAnnotatorTypeHelper {
    lt_num: i32,
    has_struct_lt: bool,
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
                match &mut r.lifetime {
                    None => {
                        r.lifetime = Some(Lifetime::new(
                            format!("'lt{}", self.lt_num).as_str(),
                            Span::call_site(),
                        ));
                        self.lt_num += 1;
                    }
                    Some(lt) => {
                        if !lt.clone().ident.to_string().starts_with("lt") {
                            r.lifetime = Some(Lifetime::new(
                                format!("'lt{}", self.lt_num).as_str(),
                                Span::call_site(),
                            ));
                            self.lt_num += 1;
                        }
                    }
                }
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
                        if !lt.clone().ident.to_string().starts_with("lt") {
                            *lt = Lifetime::new(
                                format!("'lt{}", self.lt_num).as_str(),
                                Span::call_site(),
                            );
                            self.lt_num += 1;
                            self.has_struct_lt = true;
                        }
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
                                        self.has_struct_lt = true;
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
    has_struct_lt: bool,
}

impl VisitMut for LooseLifetimeAnnotatorFnArgHelper {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (), // cannot annotate self
            FnArg::Typed(t) => {
                let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                    lt_num: self.lt_num,
                    has_struct_lt: false,
                };
                type_helper.visit_type_mut(t.ty.as_mut());
                self.lt_num = type_helper.lt_num;
                if !self.has_struct_lt {
                    self.has_struct_lt = type_helper.has_struct_lt
                }
            }
        }
    }

    fn visit_angle_bracketed_generic_arguments_mut(
        &mut self,
        i: &mut AngleBracketedGenericArguments,
    ) {
        i.args.iter_mut().for_each(|arg| match arg {
            GenericArgument::Lifetime(lt) => {
                if !lt.clone().ident.to_string().starts_with("lt") {
                    *lt = Lifetime::new(format!("'lt{}", self.lt_num).as_str(), Span::call_site());
                    self.lt_num += 1;
                    self.has_struct_lt = true;
                }
            }
            gen => {
                let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                    lt_num: self.lt_num,
                    has_struct_lt: false,
                };
                type_helper.visit_generic_argument_mut(gen);
                self.lt_num = type_helper.lt_num;
                if !self.has_struct_lt {
                    self.has_struct_lt = type_helper.has_struct_lt
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
    has_struct_lt: bool,
}

impl VisitMut for LooseLifetimeAnnotator<'_> {
    fn visit_impl_item_method_mut(&mut self, i: &mut ImplItemMethod) {
        let id = i.sig.ident.to_string();
        //println!("caller name: {}, at: {}", self.caller_fn_name, &id);
        match id == self.fn_name.to_string() {
            false => (),
            true => self.loose_lifetime_annotator(&mut i.sig),
        }
        syn::visit_mut::visit_impl_item_method_mut(self, i);
    }

    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => self.loose_lifetime_annotator(&mut i.sig),
        }
    }

    fn visit_trait_item_method_mut(&mut self, i: &mut TraitItemMethod) {
        let id = i.sig.ident.to_string();
        //println!("caller name: {}, at: {}", self.caller_fn_name, &id);
        match id == self.fn_name.to_string() {
            false => (),
            true => self.loose_lifetime_annotator(&mut i.sig),
        }
        syn::visit_mut::visit_trait_item_method_mut(self, i);
    }
}

impl LooseLifetimeAnnotator<'_> {
    fn loose_lifetime_annotator(&mut self, sig: &mut Signature) {
        match (&mut sig.inputs, &mut sig.generics, &mut sig.output) {
            (inputs, _, _) if inputs.len() == 0 => self.success = true,
            (inputs, gen, out) => {
                inputs.iter_mut().for_each(|arg| {
                    let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper {
                        lt_num: self.lt_num,
                        has_struct_lt: false,
                    };
                    fn_arg_helper.visit_fn_arg_mut(arg);
                    self.lt_num = fn_arg_helper.lt_num;
                    if !self.has_struct_lt {
                        self.has_struct_lt = fn_arg_helper.has_struct_lt
                    }
                });
                gen.params.iter_mut().for_each(|param| {
                    let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper {
                        lt_num: self.lt_num,
                        has_struct_lt: false,
                    };
                    fn_arg_helper.visit_generic_param_mut(param);
                    self.lt_num = fn_arg_helper.lt_num;
                    if !self.has_struct_lt {
                        self.has_struct_lt = fn_arg_helper.has_struct_lt
                    }
                });
                match out {
                    ReturnType::Type(_, ty) => {
                        let mut type_helper = LooseLifetimeAnnotatorTypeHelper {
                            lt_num: self.lt_num,
                            has_struct_lt: false,
                        };
                        type_helper.visit_type_mut(ty.as_mut());
                        self.lt_num = type_helper.lt_num;
                        if !self.has_struct_lt {
                            self.has_struct_lt = type_helper.has_struct_lt
                        }
                    }
                    ReturnType::Default => {}
                };
                for lt in 0..self.lt_num {
                    let lifetime = Lifetime::new(format!("'lt{}", lt).as_str(), Span::call_site());
                    gen.params.push(syn::GenericParam::Lifetime(LifetimeDef {
                        attrs: vec![],
                        lifetime,
                        colon_token: None,
                        bounds: Default::default(),
                    }))
                }
                self.success = true
            }
        }
    }
}

struct AnnotationResult {
    success: bool,
    has_struct_lt: bool,
}

fn annotate_loose_named_lifetime(new_file_name: &str, fn_name: &str) -> AnnotationResult {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut visit = LooseLifetimeAnnotator {
        fn_name,
        success: false,
        has_struct_lt: false,
        lt_num: 0,
    };
    visit.visit_file_mut(&mut file);
    let file = file.into_token_stream().to_string();
    let success = match visit.success {
        true => {
            fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
            true
        }
        false => false,
    };

    AnnotationResult {
        success,
        has_struct_lt: visit.has_struct_lt,
    }
}
