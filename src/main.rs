fn main() {
    use egg::{rewrite as rw, *};

    define_language! {
      enum BooleanLanguage {
          "&" = And([Id; 2]),
          "|" = Or([Id; 2]),
          "!" = Not([Id; 1]),
          Num(i32),
          Symbol(Symbol),
          Other(Symbol, Vec<Id>),
      }
    }
    let rules: &[Rewrite<BooleanLanguage, ()>] = &[
        // Axioms of Boolean logic from Wikipedia + DeMorgan's Laws.
        rw!("associate-and"; "(& ?x (& ?y ?z))" => "(& (& ?x ?y) ?z)"),
        rw!("associate-or"; "(| ?x (| ?y ?z))" => "(| (| ?x ?y) ?z)"),
        rw!("commute-and"; "(& ?x ?y)" => "(& ?y ?x)"),
        rw!("commute-or"; "(| ?x ?y)" => "(| ?y ?x)"),
        rw!("distribute-and"; "(& ?x (| ?y ?z))" => "(| (& ?x ?y) (& ?x ?z))"),
        rw!("distribute-or"; "(| ?x (& ?y ?z))" => "(& (| ?x ?y) (| ?x ?z))"),
        rw!("identity-and"; "(& ?x 1)" => "?x"),
        rw!("identity-or"; "(| ?x 0)" => "?x"),
        rw!("annihilate-and"; "(& ?x 0)" => "0"),
        rw!("annihilate-or"; "(| ?x 1)" => "1"),
        rw!("idempotent-and"; "(& ?x ?x)" => "?x"),
        rw!("idempotent-or"; "(| ?x ?x)" => "?x"),
        rw!("absorb-and"; "(& ?x (| ?x ?y))" => "?x"),
        rw!("absorb-or"; "(| ?x (& ?x ?y))" => "?x"),
        rw!("complement-and"; "(& ?x (! ?x))" => "0"),
        rw!("complement-or"; "(| ?x (! ?x))" => "1"),
        rw!("not-0"; "(! 0)" => "1"),
        rw!("not-1"; "(! 1)" => "0"),
        rw!("not-not"; "(! (! ?x))" => "?x"),
        rw!("demorgan-and"; "(! (& ?x ?y))" => "(| (! ?x) (! ?y))"),
        rw!("demorgan-or"; "(! (| ?x ?y))" => "(& (! ?x) (! ?y))"),
        // Definitions of gates from cell library.
        rw!("nand2"; "(| (! ?x) (! ?y))" => "(nand2 ?x ?y)"),
    ];

    // While it may look like we are working with numbers,
    // SymbolLang stores everything as strings.
    // We can make our own Language later to work with other types.
    let start = "(! (& a b))".parse().unwrap();

    // That's it! We can run equality saturation now.
    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(&start)
        .run(rules);

    // Extractors can take a user-defined cost function,
    // we'll use the egg-provided AstSize for now
    let extractor = Extractor::new(&runner.egraph, AstSize);

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
