use syn::ItemFn;
use syn::visit_mut::VisitMut;

struct CallerVisitor<'a> {
    caller_fn_name: &'a str,
    callee_fn_name: &'a str,
}

impl VisitMut for CallerVisitor {
    fn visit_item_fn_mut(&mut self, i: &mut ItemFn) {
        let id = i.sig.ident.to_string();
        match id == self.caller_fn_name {
            false => (),
            true => {

            }
        }
    }
}

struct CalleeVisitor<'a> {
    callee_fn_name: &'a str,

}

pub fn make_controls(
    file_name: &str,
    new_file_name: &str,
    callee_fn_name: &str,
    caller_fn_name: &str,
) {

}