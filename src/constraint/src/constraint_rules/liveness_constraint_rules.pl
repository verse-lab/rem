:- use_module(library(chr)).
:- chr_constraint live/1, def/1, use/1, move/2, alias/2.

/* Any variable defined at this point is alive at that point */
def(X) <=> dead(X).

live(X), alias(Y, X) <=> live(Y).

live(Y), move(Y, X) <=> dead(Y).

use(X) <=> live(X).