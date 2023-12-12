mod node_identification;

use std::error::Error;

use crate::node_identification::{filter_graph, export_to_graphvis, read_data};

fn main() -> Result<(), Box<dyn Error>> {
    let file_path = "amazon_cleaned.csv";
    let graph = read_data(file_path);
    let graph = filter_graph(&graph, 0.3);


    export_to_graphvis(&graph, "graph.dot");

    Ok(())
}
