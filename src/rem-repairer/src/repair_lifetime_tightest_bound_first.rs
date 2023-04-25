use proc_macro2::Span;
use quote::ToTokens;
use regex::Regex;

use log::debug;
use std::fs;
use syn::{visit_mut::VisitMut, FnArg, Lifetime, LifetimeDef, Type};

use crate::common::{
    callee_renamer, elide_lifetimes_annotations, repair_bounds_help, repair_iteration,
    repair_iteration_project, RepairResult, RepairSystem, RustcError,
};
use crate::repair_lifetime_simple;
use utils::{check_project, compile_file, format_source};

pub struct Repairer {}

impl RepairSystem for Repairer {
    fn name(&self) -> &str {
        "_tightest_bounds_first_repairer"
    }

    fn repair_project(&self, src_path: &str, manifest_path: &str, fn_name: &str) -> RepairResult {
        annotate_tight_named_lifetime(src_path, fn_name);
        let mut compile_cmd = check_project(manifest_path, &vec![]);
        let process_errors = |ce: &RustcError| {
            if repair_bounds_help(ce.rendered.as_str(), src_path, fn_name) {
                true
            } else {
                loosen_bounds(ce.rendered.as_str(), src_path, fn_name)
            }
        };
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
                    has_struct_lt: elide_res.has_struct_lt,
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
        annotate_tight_named_lifetime(&new_file_name, fn_name);
        //println!("annotated: {}", fs::read_to_string(&new_file_name).unwrap());
        let args: Vec<&str> = vec!["--error-format=json"];

        let mut compile_cmd = compile_file(&new_file_name, &args);

        let process_errors = |stderr: &str| {
            if repair_bounds_help(stderr, new_file_name, fn_name) {
                true
            } else {
                loosen_bounds(stderr, new_file_name, fn_name)
            }
        };

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

struct TightLifetimeAnnotatorTypeHelper {}

impl VisitMut for TightLifetimeAnnotatorTypeHelper {
    fn visit_type_mut(&mut self, i: &mut Type) {
        match i {
            Type::Reference(r) => {
                r.lifetime = Some(Lifetime::new("'lt0", Span::call_site()));
                self.visit_type_mut(r.elem.as_mut());
            }
            _ => (),
        }
    }
}

struct TightLifetimeAnnotatorFnArgHelper {}

impl VisitMut for TightLifetimeAnnotatorFnArgHelper {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(r) => match &mut r.reference {
                None => {}
                Some((_, lt)) => {
                    *lt = Some(Lifetime::new("'lt0", Span::call_site()));
                }
            },
            FnArg::Typed(t) => {
                let mut type_helper = TightLifetimeAnnotatorTypeHelper {};
                type_helper.visit_type_mut(t.ty.as_mut())
            }
        }
    }
}

struct TightLifetimeAnnotator<'a> {
    fn_name: &'a str,
    success: bool,
}

impl VisitMut for TightLifetimeAnnotator<'_> {
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
                    let lifetime = Lifetime::new("'lt0", Span::call_site());
                    gen.params.push(syn::GenericParam::Lifetime(LifetimeDef {
                        attrs: vec![],
                        lifetime,
                        colon_token: None,
                        bounds: Default::default(),
                    }));
                    inputs.iter_mut().for_each(|arg| {
                        let mut fn_arg_helper = TightLifetimeAnnotatorFnArgHelper {};
                        fn_arg_helper.visit_fn_arg_mut(arg)
                    });
                    match out {
                        syn::ReturnType::Type(_, ty) => match ty.as_mut() {
                            Type::Reference(r) => {
                                r.lifetime = Some(Lifetime::new("'lt0", Span::call_site()))
                            }
                            _ => (),
                        },
                        _ => (),
                    };
                    self.success = true
                }
            },
        }
    }
}

pub fn annotate_tight_named_lifetime(new_file_name: &str, fn_name: &str) -> bool {
    let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
    let mut file = syn::parse_str::<syn::File>(file_content.as_str())
        .map_err(|e| format!("{:?}", e))
        .unwrap();
    let mut visit = TightLifetimeAnnotator {
        fn_name,
        success: false,
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

struct BoundsLoosener<'a> {
    fn_name: &'a str,
    arg_name: &'a str,
    success: bool,
}

struct ArgBoundLoosener<'a> {
    arg_name: &'a str,
    lt: &'a str,
    success: bool,
}

impl VisitMut for ArgBoundLoosener<'_> {
    fn visit_fn_arg_mut(&mut self, i: &mut FnArg) {
        match i {
            FnArg::Receiver(_) => (), // don't modify receiver yet (&self)
            FnArg::Typed(t) => match t.pat.as_mut() {
                syn::Pat::Ident(id) if id.ident.to_string() == self.arg_name => {
                    match t.ty.as_mut() {
                        Type::Reference(r) => {
                            r.lifetime = Some(Lifetime::new(self.lt, Span::call_site()));
                            self.success = true
                        }
                        _ => (),
                    }
                }
                _ => (),
            },
        }
    }
}

impl VisitMut for BoundsLoosener<'_> {
    fn visit_item_fn_mut(&mut self, i: &mut syn::ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.fn_name.to_string() {
            false => (),
            true => {
                let mut lt_count = 0;
                let gen = &mut i.sig.generics;
                for i in &gen.params {
                    match i {
                        syn::GenericParam::Lifetime(LifetimeDef { .. }) => lt_count += 1,
                        _ => (),
                    }
                }
                let lt = format!("'lt{}", lt_count);
                let lifetime = Lifetime::new(lt.as_str(), Span::call_site());
                gen.params.push(syn::GenericParam::Lifetime(LifetimeDef {
                    attrs: vec![],
                    lifetime,
                    colon_token: None,
                    bounds: Default::default(),
                }));
                let mut arg_loosener = ArgBoundLoosener {
                    arg_name: self.arg_name,
                    lt: lt.as_str(),
                    success: false,
                };
                let inputs = &mut i.sig.inputs;
                inputs
                    .iter_mut()
                    .for_each(|arg| arg_loosener.visit_fn_arg_mut(arg));
                match arg_loosener.success {
                    true => self.success = true,
                    false => (),
                }
            }
        }
    }
}

pub fn loosen_bounds(stderr: &str, new_file_name: &str, fn_name: &str) -> bool {
    let deserializer = serde_json::Deserializer::from_str(stderr);
    let stream = deserializer.into_iter::<RustcError>();
    let mut helped = false;
    for item in stream {
        let rendered = match item {
            Ok(item) => item.rendered,
            Err(_) => stderr.to_string(),
        };
        let reference_re = Regex::new(r"error.*`(?P<ref_full>\**(?P<ref>[a-z]+))`").unwrap();
        let error_lines = reference_re.captures_iter(rendered.as_str());

        for captured in error_lines {
            //println!("ref_full: {}, ref: {}", &captured["ref_full"], &captured["ref"]);
            let file_content: String = fs::read_to_string(&new_file_name).unwrap().parse().unwrap();
            let mut file = syn::parse_str::<syn::File>(file_content.as_str())
                .map_err(|e| format!("{:?}", e))
                .unwrap();
            let mut visit = BoundsLoosener {
                fn_name,
                arg_name: &captured["ref"],
                success: false,
            };
            visit.visit_file_mut(&mut file);
            let file = file.into_token_stream().to_string();
            match visit.success {
                true => {
                    fs::write(new_file_name.to_string(), format_source(&file)).unwrap();
                    helped = true
                }
                false => (),
            }
        }
    }
    helped
}
