use egg_netlist_synthesizer::Synthesizer;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    let synthesizer = Synthesizer::new(&args[1], &args[2]);

    synthesizer.run(&args[3]);
}
