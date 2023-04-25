:- use_module(library(chr)).
:- chr_constraint deref/2, compat/2, offset/2, index/2, shouldindex/2, mut/1, ref/2, vec/1, malloc/1, tvec/1.

deref(OffsetPtr, Res), offset(Ptr, OffsetPtr) <=> index(Ptr, Res).

/* New rules for handling matrices */
/* If we have an array, and we dereference it, and we malloc the output, it should also be another dynamic array. */
/* unify index and shouldindex */
index(Ptr, Output) \ shouldindex(Ptr, NextOutput) <=> Output = NextOutput.
/* ref(A,B) means A refers to B. */
/* If Ptr (P) dereferences to Res (*P), and P is a reference of X, then *P = X */
deref(Ptr, Res), ref(Ptr, ReferredTo) <=> Res=ReferredTo.
/* */

offset(Ptr, OffsetPtr), offset(OffsetPtr, Result) <=> OffsetPtr = Result, offset(Ptr, Result).
index(Ptr, Output) \ deref(Ptr, Result) <=> Output = Result, index(Ptr, Result).
/* ==> since want to keep Malloc information for deciding if &mut  */
shouldindex(Ptr, _), malloc(Ptr) ==> vec(Ptr).

