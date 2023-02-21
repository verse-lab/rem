use proc_macro2::Span;
use quote::ToTokens;
use std::fs;
use syn::{visit_mut::VisitMut, FnArg, Lifetime, LifetimeDef, ReturnType, Type, TypeParamBound};

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
        //println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args: Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &str| repair_bounds_help(stderr, new_file_name, fn_name);

        match repair_iteration(&mut compile_cmd, &process_errors, true, Some(50)) {
            true => {
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
        match i {
            Type::Reference(r) => {
                r.lifetime = Some(Lifetime::new(
                    format!("'lt{}", self.lt_num).as_str(),
                    Span::call_site(),
                ));
                self.lt_num += 1;
                syn::visit_mut::visit_type_mut(self, r.elem.as_mut());
            }
            Type::TraitObject(t) => {
                println!(
                    "annotating trait obj: {}...",
                    t.clone().into_token_stream().to_string()
                );
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
            Type::Verbatim(v) => {
                let mut v_str = v.clone().to_string();
                println!("verbatim type: {}", v);
                while v_str.contains("'_") {
                    v_str = v_str.replacen("'_", format!("'lt{}", self.lt_num).as_str(), 1);
                    *v = syn::parse_str(v_str.as_str()).unwrap();
                    self.lt_num += 1;
                }
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
                (_, gen, _)
                    if gen.params.iter().any(|x| match x {
                        syn::GenericParam::Lifetime(_) => true,
                        _ => false,
                    }) =>
                {
                    self.success = false
                }
                (inputs, gen, out) => {
                    inputs.iter_mut().for_each(|arg| {
                        let mut fn_arg_helper = LooseLifetimeAnnotatorFnArgHelper {
                            lt_num: self.lt_num,
                        };
                        fn_arg_helper.visit_fn_arg_mut(arg);
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
