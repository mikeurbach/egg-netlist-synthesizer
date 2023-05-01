use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Deserialize)]
struct Cell {
    name: String,
    area: f64,
    power: f64,
    timing: f64,
    searcher: String,
    applier: String,
}

fn load_library<P: AsRef<Path>>(path: P) -> Result<HashMap<String, Cell>, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let cells: Vec<Cell> = serde_json::from_reader(reader)?;

    let mut library = HashMap::new();
    for cell in cells {
        library.insert(cell.name.clone(), cell);
    }
    Ok(library)
}

fn main() {
    use egg::*;

    let library =
        load_library("/Users/mikeu/skywater-preparation/sky130_fd_sc_hd_tt_100C_1v80.json")
            .unwrap();

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

    // Some axioms of Boolean logic. The goal is to allow exploration and
    // canonicalize towards right-associative DNF, which is how the logical
    // functions in the library are expressed.
    let mut rules: Vec<Rewrite<BooleanLanguage, ()>> = vec![
        rewrite!("associate-and"; "(& (& ?x ?y) ?z)" => "(& ?x (& ?y ?z))"),
        rewrite!("associate-or"; "(| (| ?x ?y) ?z)" => "(| ?x (| ?y ?z))"),
        rewrite!("commute-and"; "(& ?x ?y)" => "(& ?y ?x)"),
        rewrite!("commute-or"; "(| ?x ?y)" => "(| ?y ?x)"),
        rewrite!("distribute-and"; "(& ?x (| ?y ?z))" => "(| (& ?x ?y) (& ?x ?z))"),
        rewrite!("distribute-or"; "(& (| ?x ?y) (| ?x ?z))" => "(| ?x (& ?y ?z))"),
        rewrite!("demorgan-and"; "(! (& ?x ?y))" => "(| (! ?x) (! ?y))"),
        rewrite!("demorgan-or"; "(! (| ?x ?y))" => "(& (! ?x) (! ?y))"),
    ];

    // Add rewrites from the library.
    for cell in library.values() {
        rules.push(rewrite!(cell.name; {
            let searcher: Pattern<BooleanLanguage> = cell.searcher.parse().unwrap();
            searcher
        } => {
            let applier: Pattern<BooleanLanguage> = cell.applier.parse().unwrap();
            applier
        }));
    }

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

    let start = "(| (& (| a1 b1) (& a0 b0)) (& a1 (& a0 b0)))"
        .parse()
        .unwrap();

    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(&start)
        .with_hook(|runner| {
            println!("EGraph size: {}", runner.egraph.total_size());
            Ok(())
        })
        .run(&rules);

    let extractor = Extractor::new(&runner.egraph, cost_function);

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
