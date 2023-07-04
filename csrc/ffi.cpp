#include "egg-netlist-synthesizer/include/ffi.h"
#include "egg-netlist-synthesizer/src/lib.rs.h"

BooleanExpression build_module(rust::Vec<BooleanExpression> stmts) {
  return BooleanExpression{BooleanExpressionType::Module, "module", stmts};
}

BooleanExpression build_let(rust::String name, BooleanExpression expr) {
  return BooleanExpression{BooleanExpressionType::Module, name, {expr}};
}

BooleanExpression build_and(BooleanExpression lhs, BooleanExpression rhs) {
  return BooleanExpression{BooleanExpressionType::And, "&", {lhs, rhs}};
}

BooleanExpression build_or(BooleanExpression lhs, BooleanExpression rhs) {
  return BooleanExpression{BooleanExpressionType::Or, "|", {lhs, rhs}};
}

BooleanExpression build_not(BooleanExpression expr) {
  return BooleanExpression{BooleanExpressionType::Not, "!", {expr}};
}

BooleanExpression build_bit(rust::String name) {
  return BooleanExpression{BooleanExpressionType::Bit, name,
                           rust::Vec<BooleanExpression>()};
}

BooleanExpression build_symbol(rust::String name) {
  return BooleanExpression{BooleanExpressionType::Symbol, name,
                           rust::Vec<BooleanExpression>()};
}
