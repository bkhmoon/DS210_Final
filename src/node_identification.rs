use petgraph::Directed;
use petgraph::Graph;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

use petgraph::algo::tarjan_scc;
use petgraph::dot::{Config, Dot};
use petgraph::graph::NodeIndex;

#[derive(Debug, Deserialize)]
pub struct User {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Product {
    pub id: String,
    pub category: String,
    pub price: f32,
    pub name: String,
}

const MIN_COMPONENT_SIZE: usize = 5;

pub fn read_data(filename: &str) -> Graph<Product, f32, Directed> {
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    let mut graph = Graph::<Product, f32, Directed>::new();
    let mut users: HashMap<String, Vec<String>> = HashMap::new();
    let mut products = HashMap::new();

    for line in reader.lines().skip(1) {
        let line = line.unwrap();
        let mut iter = line.split(',');

        let product_id = iter.next().unwrap().to_string();
        let user_id = iter.next().unwrap().to_string();
        let category = iter.next().unwrap().to_string();
        let price = iter.next().unwrap().parse::<f32>().unwrap();
        let name = iter.next().unwrap().to_string();

        let user = User { id: user_id };
        let product = Product {
            id: product_id,
            category,
            price,
            name,
        };

        users
            .entry(user.id.clone())
            .or_insert(Vec::new())
            .push(product.id.clone());

        products
            .entry(product.id.clone())
            .or_insert_with(|| graph.add_node(product));
    }

    println!("Number of nodes in graph: {}", graph.node_count());

    for (_, prods) in users.iter() {
        for i in 0..prods.len() {
            for j in i + 1..prods.len() {
                let prod1 = prods[i].clone();
                let prod2 = prods[j].clone();

                let node1 = products.get(&prod1).unwrap();
                let node2 = products.get(&prod2).unwrap();

                if node1 == node2 {
                    continue;
                }

                if graph.contains_edge(*node1, *node2) {
                    let edge = graph.find_edge(*node1, *node2).unwrap();
                    let weight = graph.edge_weight_mut(edge).unwrap();
                    *weight += 1.0;
                } else {
                    graph.add_edge(*node1, *node2, 1.0);
                }
            }
        }
    }

    graph
}

fn remove_small_components(graph: &mut Graph<Product, f32, Directed>, scc: &Vec<Vec<NodeIndex>>) {
    for component in scc {
        if component.len() < 3 {
            for &node in component {
                graph.remove_node(node);
            }
        }
    }
}

pub fn filter_graph(
    graph: &Graph<Product, f32, Directed>,
    coefficient: f32,
) -> Graph<Product, f32, Directed> {
    // Assuming `graph` is your previously created graph instance
    let mut graph = graph.clone();

    let max_iter = 100;
    let mut iter = 0;
    loop {
        // Compute the strongly connected components of the graph
        let scc: Vec<Vec<NodeIndex>> = tarjan_scc(&graph);

        // Create a map to hold nodes by their component
        let mut component_map: HashMap<usize, Vec<NodeIndex>> = HashMap::new();

        // Group nodes by their component
        for (i, component) in scc.iter().enumerate() {
            for &node in component {
                component_map.entry(i).or_insert(Vec::new()).push(node);
            }
        }

        remove_small_components(&mut graph, &scc);

        iter += 1;
        if iter == max_iter
            || component_map.values().map(|v| v.len()).min().unwrap() >= MIN_COMPONENT_SIZE
        {
            println!("Number of connected components: {}", scc.len());

            println!(
                "Number of nodes in the largest connected component: {}",
                component_map.values().map(|v| v.len()).max().unwrap()
            );

            println!(
                "Number of nodes in the smallest connected component: {}",
                component_map.values().map(|v| v.len()).min().unwrap()
            );
            break;
        }
    }

    let mut nodes_to_remove = vec![];
    let scc: Vec<Vec<NodeIndex>> = tarjan_scc(&graph);
    for component in scc {
        let coeff = component_clustering_coefficient(&graph, &component);

        if coeff < coefficient as f64 {
            for &node in &component {
                nodes_to_remove.push(node);
            }
        }
    }

    for node in nodes_to_remove {
        graph.remove_node(node);
    }

    let mut scc: Vec<Vec<NodeIndex>> = tarjan_scc(&graph);
    scc.sort_by(|a, b| {
        component_clustering_coefficient(&graph, b)
            .partial_cmp(&component_clustering_coefficient(&graph, a))
            .unwrap()
    });

    for component in scc {
        if component.len() < MIN_COMPONENT_SIZE {
            continue;
        }

        println!(
            "Component size: {}, Coefficient: {}",
            component.len(),
            component_clustering_coefficient(&graph, &component)
        );
    }

    graph
}

pub fn export_to_graphvis(graph: &Graph<Product, f32, Directed>, filename: &str) {
    let viz = Dot::with_attr_getters(
        graph,
        &[Config::EdgeNoLabel],
        &|_, _| "label=\"\"".to_string(),
        &|_, (_, node)| {
            let label = format!("{}: {}", node.category, node.price);
            format!("label=\"{}\"", label)
        },
    );

    std::fs::write(filename, format!("{:?}", viz)).unwrap();
}

// Function to calculate the clustering coefficient for a node
fn clustering_coefficient(graph: &Graph<Product, f32, Directed>, node: NodeIndex) -> f64 {
    let neighbors: Vec<_> = graph.neighbors(node).collect();
    let mut edges_between_neighbors = 0;

    for (i, &ni) in neighbors.iter().enumerate() {
        for &nj in &neighbors[i + 1..] {
            if graph.contains_edge(ni, nj) {
                edges_between_neighbors += 1;
            }
        }
    }

    if neighbors.len() < 2 {
        return 0.0;
    }

    let total_possible_edges = neighbors.len() * (neighbors.len() - 1) / 2;
    if total_possible_edges > 0 {
        edges_between_neighbors as f64 / total_possible_edges as f64
    } else {
        0.0
    }
}

// Function to calculate the average clustering coefficient for a component
fn component_clustering_coefficient(
    graph: &Graph<Product, f32, Directed>,
    component: &[NodeIndex],
) -> f64 {
    let mut total_coefficient = 0.0;
    for &node in component {
        total_coefficient += clustering_coefficient(graph, node);
    }
    total_coefficient / component.len() as f64
}

//test module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_data() {
        let graph = read_data("amazon_cleaned.csv");
        assert_eq!(graph.node_count(), 1351);
        assert_eq!(graph.edge_count(), 761);
    }

    #[test]
    fn test_component_clustering_coefficient() {
        let mut graph = Graph::<Product, f32, Directed>::new();

        let node1 = graph.add_node(Product {
            id: "1".to_string(),
            category: "cat1".to_string(),
            price: 1.0,
            name: "prod1".to_string(),
        });

        let node2 = graph.add_node(Product {
            id: "2".to_string(),
            category: "cat1".to_string(),
            price: 1.0,
            name: "prod2".to_string(),
        });

        let node3 = graph.add_node(Product {
            id: "3".to_string(),
            category: "cat1".to_string(),
            price: 1.0,
            name: "prod3".to_string(),
        });

        graph.add_edge(node1, node2, 1.0);
        graph.add_edge(node1, node2, 1.0);
        graph.add_edge(node1, node3, 1.0);
        graph.add_edge(node2, node1, 1.0);
        graph.add_edge(node2, node3, 1.0);
        graph.add_edge(node3, node2, 1.0);

        let scc: Vec<Vec<NodeIndex>> = tarjan_scc(&graph);

        let coeff = component_clustering_coefficient(&graph, &scc[0]);
        assert_eq!(coeff, 0.2222222222222222);
    }
}
