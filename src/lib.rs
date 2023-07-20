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
      "input" = Input([Id; 2]),
      "output" = Output(Id),
      Num(i32),
      Symbol(Symbol),
      Gate(Symbol, Vec<Id>),
  }
}

// Use the newtype idiom to define types for BooleanLanguage.

pub struct BooleanExpression(pub RecExpr<BooleanLanguage>);
pub struct BooleanEGraph(pub EGraph<BooleanLanguage, ()>);
struct BooleanId(Id);

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
            BooleanLanguage::Input(_) => 0.0,
            BooleanLanguage::Output(_) => 0.0,
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

    pub fn run(
        &self,
        mut egraph: BooleanEGraph,
        start_expr: BooleanExpression,
    ) -> BooleanExpression {
        // Ensure the EGraph is ready after any mutations.
        egraph.0.rebuild();

        // Run the optimizer with some debug info.
        let mut runner = Runner::default()
            .with_explanations_enabled()
            .with_egraph(egraph.0)
            .with_expr(&start_expr.0)
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
                .explain_equivalence(&start_expr.0, &best_expr)
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

        BooleanExpression(best_expr)
    }
}

/// C++ FFI.

// EGraph API.
fn egraph_new() -> Box<BooleanEGraph> {
    let egraph = EGraph::<BooleanLanguage, ()>::default().with_explanations_enabled();
    Box::new(BooleanEGraph(egraph))
}

// Synthesizer API.

fn synthesizer_new(library_path: String, metric_name: String) -> Box<Synthesizer> {
    Box::new(Synthesizer::new(
        library_path.as_str(),
        metric_name.as_str(),
    ))
}

fn synthesizer_run(
    egraph: Box<BooleanEGraph>,
    synthesizer: Box<Synthesizer>,
    expr: Box<BooleanExpression>,
) -> Box<BooleanExpression> {
    let best_expr = (*synthesizer).run(*egraph, *expr);
    Box::new(best_expr)
}

// Expression builders.

fn build_module(egraph: &mut BooleanEGraph, stmts: Vec<BooleanId>) -> Box<BooleanExpression> {
    let mut stmt_ids: Vec<Id> = vec![];
    for stmt in stmts {
        stmt_ids.push(stmt.0);
    }
    let enode = BooleanLanguage::Module(stmt_ids);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanExpression(egraph.0.id_to_expr(expr_id)))
}

fn build_let(egraph: &mut BooleanEGraph, name: String, expr: Box<BooleanId>) -> Box<BooleanId> {
    let name_symbol = build_symbol(egraph, name);
    let enode = BooleanLanguage::Let([name_symbol.0, expr.0]);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn build_and(
    egraph: &mut BooleanEGraph,
    lhs: Box<BooleanId>,
    rhs: Box<BooleanId>,
) -> Box<BooleanId> {
    let enode = BooleanLanguage::And([lhs.0, rhs.0]);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn build_or(
    egraph: &mut BooleanEGraph,
    lhs: Box<BooleanId>,
    rhs: Box<BooleanId>,
) -> Box<BooleanId> {
    let enode = BooleanLanguage::Or([lhs.0, rhs.0]);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn build_not(egraph: &mut BooleanEGraph, input: Box<BooleanId>) -> Box<BooleanId> {
    let enode = BooleanLanguage::Not([input.0]);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn build_num(egraph: &mut BooleanEGraph, num: i32) -> Box<BooleanId> {
    let enode = BooleanLanguage::Num(num);
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn build_symbol(egraph: &mut BooleanEGraph, name: String) -> Box<BooleanId> {
    let enode = BooleanLanguage::Symbol(Symbol::from(name));
    let expr_id = egraph.0.add(enode);
    Box::new(BooleanId(expr_id))
}

fn expr_get_module_body(expr: Box<BooleanExpression>) -> Vec<BooleanExpression> {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Module(ids) => {
            let mut exprs = vec![];
            for id in ids {
                let child_node = expr.0[*id].clone();
                let child_expr = child_node.build_recexpr(|id| expr.0[id].clone());
                exprs.push(BooleanExpression(child_expr));
            }
            exprs
        }
        _ => panic!("expected expr to be a module"),
    }
}

fn expr_get_let_symbol(expr: &BooleanExpression) -> String {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Let([symbol_id, _]) => {
            let symbol_node = expr.0[*symbol_id].clone();
            match symbol_node {
                BooleanLanguage::Symbol(symbol) => symbol.to_string(),
                _ => panic!("expected first child of let to be a symbol"),
            }
        }
        _ => panic!("expected expr to be a let"),
    }
}

fn expr_get_let_expr(expr: &BooleanExpression) -> Box<BooleanExpression> {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Let([_, child_expr_id]) => {
            let child_expr_node = expr.0[*child_expr_id].clone();
            let child_expr = child_expr_node.build_recexpr(|id| expr.0[id].clone());
            Box::new(BooleanExpression(child_expr))
        }
        _ => panic!("expected expr to be a let"),
    }
}

fn expr_get_gate_name(expr: &BooleanExpression) -> String {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Gate(symbol, _) => symbol.to_string(),
        _ => panic!("expected expr to be a gate"),
    }
}

fn expr_get_gate_input_names(expr: &BooleanExpression) -> Vec<String> {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Gate(_, pin_ids) => {
            let mut names = vec![];
            for id in pin_ids {
                let pin_node = expr.0[*id].clone();
                match pin_node {
                    BooleanLanguage::Input([symbol_id, _]) => {
                        let symbol_node = expr.0[symbol_id].clone();
                        match symbol_node {
                            BooleanLanguage::Symbol(symbol) => names.push(symbol.to_string()),
                            _ => panic!("expected first child of Input to be a symbol"),
                        }
                    }
                    _ => (),
                }
            }
            names
        }
        _ => panic!("expected expr to be a gate"),
    }
}

fn expr_get_gate_input_exprs(expr: &BooleanExpression) -> Vec<BooleanExpression> {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Gate(_, pin_ids) => {
            let mut exprs = vec![];
            for id in pin_ids {
                let pin_node = expr.0[*id].clone();
                match pin_node {
                    BooleanLanguage::Input([_, expr_id]) => {
                        let input_expr_node = expr.0[expr_id].clone();
                        let input_expr = input_expr_node.build_recexpr(|id| expr.0[id].clone());
                        exprs.push(BooleanExpression(input_expr))
                    }
                    _ => (),
                }
            }
            exprs
        }
        _ => panic!("expected expr to be a gate"),
    }
}

fn expr_get_gate_output_name(expr: &BooleanExpression) -> String {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Gate(_, pin_ids) => {
            let mut names = vec![];
            for id in pin_ids {
                let pin_node = expr.0[*id].clone();
                match pin_node {
                    BooleanLanguage::Output(symbol_id) => {
                        let symbol_node = expr.0[symbol_id].clone();
                        match symbol_node {
                            BooleanLanguage::Symbol(symbol) => names.push(symbol.to_string()),
                            _ => panic!("expected first child of Output to be a symbol"),
                        }
                    }
                    _ => (),
                }
            }
            names[0].clone()
        }
        _ => panic!("expected expr to be a gate"),
    }
}

fn expr_is_symbol(expr: &BooleanExpression) -> bool {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Symbol(_) => true,
        _ => false,
    }
}

fn expr_get_symbol(expr: &BooleanExpression) -> String {
    let expr_ref = expr.0.as_ref();
    match expr_ref.last().unwrap() {
        BooleanLanguage::Symbol(symbol) => symbol.to_string(),
        _ => panic!("expected expr to be a symbol"),
    }
}

fn append_expr(stmts: &mut Vec<BooleanId>, expr: Box<BooleanId>) -> () {
    stmts.push(*expr);
}

fn print_expr(expr: &mut BooleanExpression) -> () {
    println!("{}", expr.0.pretty(80));
}

#[cxx::bridge]
mod ffi {
    extern "Rust" {
        type BooleanExpression;
        type BooleanId;
        type Synthesizer;
        type BooleanEGraph;

        // EGraph API.
        fn egraph_new() -> Box<BooleanEGraph>;

        // Synthesizer API.
        fn synthesizer_new(library_path: String, metric_name: String) -> Box<Synthesizer>;

        fn synthesizer_run(
            egraph: Box<BooleanEGraph>,
            synthesizer: Box<Synthesizer>,
            expr: Box<BooleanExpression>,
        ) -> Box<BooleanExpression>;

        // Expression builders.
        fn build_module(
            egraph: &mut BooleanEGraph,
            stmts: Vec<BooleanId>,
        ) -> Box<BooleanExpression>;

        fn build_let(
            egraph: &mut BooleanEGraph,
            name: String,
            expr: Box<BooleanId>,
        ) -> Box<BooleanId>;

        fn build_and(
            egraph: &mut BooleanEGraph,
            lhs: Box<BooleanId>,
            rhs: Box<BooleanId>,
        ) -> Box<BooleanId>;

        fn build_or(
            egraph: &mut BooleanEGraph,
            lhs: Box<BooleanId>,
            rhs: Box<BooleanId>,
        ) -> Box<BooleanId>;

        fn build_not(egraph: &mut BooleanEGraph, input: Box<BooleanId>) -> Box<BooleanId>;

        fn build_num(egraph: &mut BooleanEGraph, num: i32) -> Box<BooleanId>;

        fn build_symbol(egraph: &mut BooleanEGraph, name: String) -> Box<BooleanId>;

        // Expression query functions.
        fn expr_get_module_body(expr: Box<BooleanExpression>) -> Vec<BooleanExpression>;

        fn expr_get_let_symbol(expr: &BooleanExpression) -> String;

        fn expr_get_let_expr(expr: &BooleanExpression) -> Box<BooleanExpression>;

        fn expr_get_gate_name(expr: &BooleanExpression) -> String;

        fn expr_get_gate_input_names(expr: &BooleanExpression) -> Vec<String>;

        fn expr_get_gate_input_exprs(expr: &BooleanExpression) -> Vec<BooleanExpression>;

        fn expr_get_gate_output_name(expr: &BooleanExpression) -> String;

        fn expr_is_symbol(expr: &BooleanExpression) -> bool;

        fn expr_get_symbol(expr: &BooleanExpression) -> String;

        // Helpers.
        fn append_expr(stmts: &mut Vec<BooleanId>, expr: Box<BooleanId>);

        fn print_expr(expr: &mut BooleanExpression) -> ();
    }
}
