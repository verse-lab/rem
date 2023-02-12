:- use_module(library(chr)).
:- chr_constraint alias/2, assign/2, ref/1.

/* propagating the aliasing */
alias(Q, P), alias(R, Q) ==> alias(R, P).

/* if P is reference type and Q must be a reference type (and vice versa)
then assigning Q = P means aliasing P to Q */
ref(P) \ assign(Q, P) <=> alias(Q, P).
ref(Q) \ assign(Q, P) <=> alias(Q, P).
