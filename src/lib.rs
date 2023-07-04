use egg::*;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
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
  pub enum BooleanLanguage {
      "module" = Module(Vec<Id>),
      "let" = Let([Id; 2]),
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
// the logic, then map to gates. Symbols are free to encourage reusing let
// expressions when possible. Among gates, the relative cost is dictated by the
// chosen metric and the cell library.

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

struct GateCostFunction {
    metric: Metric,
    library: HashMap<String, Cell>,
}

impl GateCostFunction {
    fn gate_cost(&self, name: &Symbol) -> f64 {
        let cell = self.library.get(&name.to_string()).unwrap();
        match self.metric {
            Metric::Area => cell.area,
            Metric::Power => cell.power,
            Metric::Timing => cell.timing,
        }
    }
}

impl LpCostFunction<BooleanLanguage, ()> for &GateCostFunction {
    fn node_cost(
        &mut self,
        egraph: &EGraph<BooleanLanguage, ()>,
        _eclass: Id,
        enode: &BooleanLanguage,
    ) -> f64 {
        // Cost function for each ENode.
        let op_cost = match enode {
            BooleanLanguage::And(_) => 1000000000.0,
            BooleanLanguage::Or(_) => 1000000000.0,
            BooleanLanguage::Not(_) => 1000000000.0,
            BooleanLanguage::Gate(name, _) => self.gate_cost(name),
            BooleanLanguage::Module(_) => 0.0,
            BooleanLanguage::Let(_) => 0.0,
            BooleanLanguage::Num(_) => 0.0,
            BooleanLanguage::Symbol(_) => 0.0,
        };

        // Compute the cost of a subtree of expressions by taking the minimum
        // cost of all the ENodes in each child EClass.
        enode.fold(op_cost, |sum, id| {
            let mut min_cost = 1000000000.0;
            for child_enode in &egraph[id].nodes {
                let child_cost = self.node_cost(egraph, id, child_enode);
                if child_cost < min_cost {
                    min_cost = child_cost;
                }
            }
            sum + min_cost
        })
    }
}

pub struct Synthesizer {
    rules: Vec<Rewrite<BooleanLanguage, ()>>,
    cost_function: GateCostFunction,
}

impl Synthesizer {
    pub fn new(library_path: &str, metric_name: &str) -> Synthesizer {
        let library = load_library(library_path).unwrap();
        let metric = Metric::from_str(metric_name).unwrap();

        // Some axioms of Boolean logic. The goal is to allow exploration and
        // canonicalize towards right-associative DNF, which is how the logical
        // functions in the library are expressed.
        let mut rules: Vec<Rewrite<BooleanLanguage, ()>> = vec![
            rewrite!("commute-and"; "(& ?x ?y)" => "(& ?y ?x)"),
            rewrite!("commute-or"; "(| ?x ?y)" => "(| ?y ?x)"),
            rewrite!("demorgan-and"; "(! (& ?x ?y))" => "(| (! ?x) (! ?y))"),
            rewrite!("demorgan-or"; "(! (| ?x ?y))" => "(& (! ?x) (! ?y))"),
            multi_rewrite!("inline-let-and"; "?a = (let ?x ?y), ?b = (& ?x ?z)" => "?b = (& ?y ?z)"),
            multi_rewrite!("inline-let-or"; "?a = (let ?x ?y), ?b = (| ?x ?z)" => "?b = (| ?y ?z)"),
            multi_rewrite!("inline-let-not"; "?a = (let ?x ?y), ?b = (! ?x)" => "?b = (! ?y)"),
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
            library: library,
        };

        Synthesizer {
            rules: rules,
            cost_function: cost_function,
        }
    }

    pub fn run(&self, start: RecExpr<BooleanLanguage>) {
        // Run the optimizer with some debug info.
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_expr(&start)
            .run(&self.rules);

        // Instantiate an extractor.
        let mut extractor = LpExtractor::new(&runner.egraph, &self.cost_function);

        // Extract the best expression.
        let best_expr = extractor.solve(runner.roots[0]);

        // Let explanations mutably borrow the runner.
        drop(extractor);

        // Provide some debug output.
        runner.print_report();

        println!(
            "Explanation\n===========\n{}",
            runner
                .explain_equivalence(&start, &best_expr)
                .get_flat_string()
        );

        println!("\nResult\n======\n{}", best_expr);

        // Produce a visualization of the EGraph.
        runner
            .egraph
            .dot()
            .with_config_line("ranksep=1")
            .to_svg("egraph.svg")
            .unwrap();
    }
}

/// C++ FFI.

#[cxx::bridge]
mod ffi {
    struct BooleanExpression {
        tpe: BooleanExpressionType,
        name: String,
        children: Vec<BooleanExpression>,
    }

    enum BooleanExpressionType {
        Module,
        Let,
        And,
        Or,
        Not,
        Bit,
        Symbol,
        Gate,
    }

    unsafe extern "C++" {
        include!("egg-netlist-synthesizer/include/ffi.h");

        fn build_module(stmts: Vec<BooleanExpression>) -> BooleanExpression;
        fn build_let(name: String, expr: BooleanExpression) -> BooleanExpression;
        fn build_and(lhs: BooleanExpression, rhs: BooleanExpression) -> BooleanExpression;
        fn build_or(lhs: BooleanExpression, rhs: BooleanExpression) -> BooleanExpression;
        fn build_not(expr: BooleanExpression) -> BooleanExpression;
        fn build_bit(name: String) -> BooleanExpression;
        fn build_symbol(name: String) -> BooleanExpression;
    }
}
