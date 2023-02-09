:- use_module(library(chr)).
:- chr_constraint alias/2.

/* propagating the aliasing */
alias(P, Q), alias(Q, R) ==> alias(P, R).
