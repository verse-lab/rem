use std::any::Any;
use std::any::TypeId;
use std::borrow::Borrow;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Index;

use ena::unify::UnifyKey;

// https://stackoverflow.com/questions/64838355/how-do-i-create-a-hashmap-with-type-erased-keys
/// Type erasing keys
pub trait ASTKey {
    fn eq(&self, other: &dyn ASTKey) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}

impl<T: Eq + Hash + 'static> ASTKey for T {
    fn eq(&self, other: &dyn ASTKey) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }
    fn hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        Hash::hash(&(TypeId::of::<T>(), self), &mut h);
        h.finish()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<'a> PartialEq for &'a dyn ASTKey {
    fn eq(&self, other: &Self) -> bool {
        ASTKey::eq(*self, *other)
    }
}

impl<'a> Eq for &'a dyn ASTKey {}

impl<'a> Hash for &'a dyn ASTKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let key_hash = ASTKey::hash(*self);
        state.write_u64(key_hash)
    }
}

fn eq_box(elt_a: &Box<dyn ASTKey>, elt_b: &Box<dyn ASTKey>) -> bool {
    ASTKey::eq(&*elt_a, &*elt_b.borrow())
}

fn hash_box<H: Hasher>(elt: &Box<dyn ASTKey>, state: &mut H) {
    Hash::hash(&elt, state)
}

impl Hash for Box<dyn ASTKey> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_box(self, state)
    }
}

impl PartialEq for Box<dyn ASTKey> {
    fn eq(&self, other: &Self) -> bool {
        eq_box(self, other)
    }
}

impl Eq for Box<dyn ASTKey> {}

/// Generic Scoped Context, maps identifiers to labels
#[derive(Debug)]
pub struct ScopedContext<K: Eq + Hash, L>(Vec<HashMap<K, L>>);

impl<K: Eq + Hash, L> Default for ScopedContext<K, L> {
    fn default() -> Self {
        ScopedContext(vec![HashMap::new()])
    }
}

impl<'a, 'b, K: Eq + Hash, L> Index<&'a K> for ScopedContext<K, &'b L> {
    type Output = L;

    fn index(&self, index: &'a K) -> &'b Self::Output {
        self.lookup(index).unwrap()
    }
}

impl<K: Eq + Hash, L> ScopedContext<K, L> {
    pub fn open_scope(&mut self) {
        self.0.push(HashMap::new())
    }
    pub fn close_scope(&mut self) {
        self.0.pop();
    }

    pub fn add_binding(&mut self, var: K, value: L) {
        self.0.last_mut().unwrap().insert(var, value);
    }
}

impl<K: Eq + Hash, L: Clone> ScopedContext<K, L> {
    pub fn lookup(&self, ident: &K) -> Option<L> {
        for table in self.0.iter().rev() {
            match table.get(ident) {
                Some(result) => return Some(result.clone()),
                _ => (),
            }
        }
        None
    }
}

/// Type encoding labels of an AST
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Label(usize);

impl std::fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "A{}", self.0)
    }
}

impl Label {
    pub fn new() -> Self {
        Label(0)
    }
    pub fn incr(&mut self) {
        self.0 += 1
    }
    pub fn of_raw(v: usize) -> Self {
        Label(v)
    }
    pub fn to_raw(self) -> usize {
        self.0
    }
}

impl UnifyKey for Label {
    type Value = Option<crate::typ::RustType>;

    fn index(&self) -> u32 {
        self.0 as u32
    }

    fn from_index(u: u32) -> Self {
        Label(u as usize)
    }

    fn tag() -> &'static str {
        todo!()
    }
}
