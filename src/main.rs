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
        rw!("commute-and"; "(& ?x ?y)" => "(& ?y ?x)"),
        rw!("commute-or"; "(| ?x ?y)" => "(| ?y ?x)"),
        rw!("demorgan-and"; "(! (& ?x ?y))" => "(| (! ?x) (! ?y))"),
        rw!("demorgan-or"; "(! (| ?x ?y))" => "(& (! ?x) (! ?y))"),
        rw!("and-0"; "(& ?x 0)" => "0"),
        rw!("and-1"; "(& ?x 1)" => "?x"),
        rw!("or-0"; "(| ?x 0)" => "?x"),
        rw!("or-1"; "(| ?x 1)" => "1"),
        rw!("not-0"; "(! 0)" => "1"),
        rw!("not-1"; "(! 1)" => "0"),
        rw!("not-not"; "(! (! ?x))" => "?x"),
        rw!("nand2"; "(| (! ?x) (! ?y))" => "nand2"),
    ];

    // While it may look like we are working with numbers,
    // SymbolLang stores everything as strings.
    // We can make our own Language later to work with other types.
    let start = "(! (& A B))".parse().unwrap();

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
