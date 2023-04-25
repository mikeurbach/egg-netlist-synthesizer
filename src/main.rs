fn main() {
    use egg::*;

    define_language! {
      enum BooleanLanguage {
          "&" = And([Id; 2]),
          "|" = Or([Id; 2]),
          "!" = Not([Id; 1]),
          Num(i32),
          Symbol(Symbol),
          Gate(Symbol, Vec<Id>),
      }
    }

    let rules: &[Rewrite<BooleanLanguage, ()>] = &[
        // Axioms of Boolean logic from Wikipedia + DeMorgan's Laws.
        rewrite!("associate-and"; "(& ?x (& ?y ?z))" => "(& (& ?x ?y) ?z)"),
        rewrite!("associate-or"; "(| ?x (| ?y ?z))" => "(| (| ?x ?y) ?z)"),
        rewrite!("commute-and"; "(& ?x ?y)" => "(& ?y ?x)"),
        rewrite!("commute-or"; "(| ?x ?y)" => "(| ?y ?x)"),
        rewrite!("distribute-and"; "(& ?x (| ?y ?z))" => "(| (& ?x ?y) (& ?x ?z))"),
        rewrite!("distribute-or"; "(| ?x (& ?y ?z))" => "(& (| ?x ?y) (| ?x ?z))"),
        rewrite!("identity-and"; "(& ?x 1)" => "?x"),
        rewrite!("identity-or"; "(| ?x 0)" => "?x"),
        rewrite!("annihilate-and"; "(& ?x 0)" => "0"),
        rewrite!("annihilate-or"; "(| ?x 1)" => "1"),
        rewrite!("idempotent-and"; "(& ?x ?x)" => "?x"),
        rewrite!("idempotent-or"; "(| ?x ?x)" => "?x"),
        rewrite!("absorb-and"; "(& ?x (| ?x ?y))" => "?x"),
        rewrite!("absorb-or"; "(| ?x (& ?x ?y))" => "?x"),
        rewrite!("complement-and"; "(& ?x (! ?x))" => "0"),
        rewrite!("complement-or"; "(| ?x (! ?x))" => "1"),
        rewrite!("not-0"; "(! 0)" => "1"),
        rewrite!("not-1"; "(! 1)" => "0"),
        rewrite!("not-not"; "(! (! ?x))" => "?x"),
        rewrite!("demorgan-and"; "(! (& ?x ?y))" => "(| (! ?x) (! ?y))"),
        rewrite!("demorgan-or"; "(! (| ?x ?y))" => "(& (! ?x) (! ?y))"),
        // Definitions of gates from cell library.
        rewrite!("and2"; "(& ?x ?y)" => "(and2 ?x ?y)"),
        rewrite!("or2"; "(| ?x ?y)" => "(or2 ?x ?y)"),
        rewrite!("nand2"; "(| (! ?x) (! ?y))" => "(nand2 ?x ?y)"),
        rewrite!("xor2"; "(| (& ?x (! ?y)) (& (! ?x) ?y))" => "(xor2 ?x ?y)"),
    ];

    // A simply cost function that prefers gates over boolean logic, and
    // literals or symbols the most. Otherwise, this is basically counting up
    // the expression size. This is intended to push the search to optimize the
    // logic, then map to gates.
    struct GateCostFunction;
    impl CostFunction<BooleanLanguage> for GateCostFunction {
        type Cost = i32;

        fn cost<C>(&mut self, enode: &BooleanLanguage, mut costs: C) -> Self::Cost
        where
            C: FnMut(Id) -> Self::Cost,
        {
            let op_cost = match enode {
                BooleanLanguage::And(_) => 2,
                BooleanLanguage::Or(_) => 2,
                BooleanLanguage::Not(_) => 2,
                BooleanLanguage::Gate(_, _) => 1,
                BooleanLanguage::Num(_) => 0,
                BooleanLanguage::Symbol(_) => 0,
            };
            enode.fold(op_cost, |sum, id| sum + costs(id))
        }
    }

    let cost_function = GateCostFunction {};

    // While it may look like we are working with numbers,
    // SymbolLang stores everything as strings.
    // We can make our own Language later to work with other types.
    let start = "(| (& (| a1 b1) (and a0 b0)) (& a1 (& a0 b0)))"
        .parse()
        .unwrap();

    // That's it! We can run equality saturation now.
    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(&start)
        .run(rules);

    // Extractors can take a user-defined cost function,
    // we'll use the egg-provided AstSize for now
    let extractor = Extractor::new(&runner.egraph, cost_function);

    // We want to extract the best expression represented in the
    // same e-class as our initial expression, not from the whole e-graph.
    // Luckily the runner stores the eclass Id where we put the initial expression.
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

    println!(
        "explanation: {}",
        runner
            .explain_equivalence(&start, &best_expr)
            .get_flat_string()
    );
    println!("best expr: {}", best_expr);
    println!("best cost: {}", best_cost);
}
