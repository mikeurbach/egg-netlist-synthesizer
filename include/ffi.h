#include "rust/cxx.h"

struct BooleanExpression;

BooleanExpression build_module(rust::Vec<BooleanExpression> stmts);
BooleanExpression build_let(rust::String name, BooleanExpression expr);
BooleanExpression build_and(BooleanExpression lhs, BooleanExpression rhs);
BooleanExpression build_or(BooleanExpression lhs, BooleanExpression rhs);
BooleanExpression build_not(BooleanExpression expr);
BooleanExpression build_bit(rust::String name);
BooleanExpression build_symbol(rust::String name);
