use egg::*;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::str::FromStr;

// Represents a cell in a library.

#[derive(Deserialize)]
struct Cell {
    name: String,
    area: f64,
    power: f64,
    timing: f64,
    searcher: String,
    applier: String,
}

// Load a library of cells from disk.

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

// A simple language for boolean logic and logic gates.

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

// A simpl cost function that prefers gates over boolean logic, and
// literals or symbols the most. This is intended to push the search to optimize
// the logic, then map to gates. Among gates, the relative cost is dictated by
// the chosen metric and the cell library.

enum Metric {
    Area,
    Power,
    Timing,
}

impl FromStr for Metric {
    type Err = ();

    fn from_str(input: &str) -> Result<Metric, Self::Err> {
        match input {
            "Area" => Ok(Metric::Area),
            "Power" => Ok(Metric::Power),
            "Timing" => Ok(Metric::Timing),
            _ => Err(()),
        }
    }
}

struct GateCostFunction<'a> {
    metric: Metric,
    library: &'a HashMap<String, Cell>,
}

impl GateCostFunction<'_> {
    fn gate_cost(&self, name: &Symbol) -> f64 {
        let cell = self.library.get(&name.to_string()).unwrap();
        match self.metric {
            Metric::Area => cell.area,
            Metric::Power => cell.power,
            Metric::Timing => cell.timing,
        }
    }
}

impl CostFunction<BooleanLanguage> for GateCostFunction<'_> {
    type Cost = f64;

    fn cost<C>(&mut self, enode: &BooleanLanguage, mut costs: C) -> Self::Cost
    where
        C: FnMut(Id) -> Self::Cost,
    {
        let op_cost = match enode {
            BooleanLanguage::And(_) => 1000000000.0,
            BooleanLanguage::Or(_) => 1000000000.0,
            BooleanLanguage::Not(_) => 1000000000.0,
            BooleanLanguage::Gate(name, _) => self.gate_cost(name),
            BooleanLanguage::Num(_) => 0.0,
            BooleanLanguage::Symbol(_) => 0.0,
        };
        enode.fold(op_cost, |sum, id| sum + costs(id))
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let library = load_library(&args[1]).unwrap();
    let metric = Metric::from_str(&args[2]).unwrap();

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

    // Instantiate our cost function with the provided metric and libary.
    let cost_function = GateCostFunction {
        metric: metric,
        library: &library,
    };

    // The start expression to synthesize.
    let start = "(| (& (| a1 b1) (& a0 b0)) (& a1 (& a0 b0)))"
        .parse()
        .unwrap();

    // Run the optimizer with some debug info.
    let mut runner = Runner::default()
        .with_explanations_enabled()
        .with_expr(&start)
        .run(&rules);

    // Instantiate an extractor.
    let extractor = Extractor::new(&runner.egraph, cost_function);

    // Extract the best expression.
    let (best_cost, best_expr) = extractor.find_best(runner.roots[0]);

    // Provide some debug output.
    runner.print_report();

    println!(
        "Explanation\n===========\n{}",
        runner
            .explain_equivalence(&start, &best_expr)
            .get_flat_string()
    );

    println!("\nCost\n====\n{}", best_cost);
}
