use egg::{EGraph, RecExpr};
use egg_netlist_synthesizer::{BooleanEGraph, BooleanExpression, BooleanLanguage, Synthesizer};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let synthesizer = Synthesizer::new(&args[1], &args[2]);

    let expr: RecExpr<BooleanLanguage> = args[3].parse().unwrap();

    let egraph = EGraph::<BooleanLanguage, ()>::default().with_explanations_enabled();

    synthesizer.run(BooleanEGraph(egraph), BooleanExpression(expr));

    ()
}
