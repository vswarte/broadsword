use std::fmt;
use graphviz_rust::attributes::SubgraphAttributes;
use graphviz_rust::dot_generator::*;
use graphviz_rust::dot_structures::*;
use graphviz_rust::printer::{DotPrinter, PrinterContext};
use crate::analysis;
use crate::analysis::NodeType;

pub(crate) struct GraphEdge {
    pub from: String,
    pub to: String,
}

pub(crate) fn build_dotviz(root: analysis::Node, edges: Vec<GraphEdge>) -> String {
    let nodes = map_node(&root);

    let edge_statements = edges.iter()
        .map(|e| {
            let from = escape(&e.from);
            let to = escape(&e.to);

            stmt!(edge!(
                node_id!(from) => node_id!(to)
            ))
        })
        .collect::<Vec<Stmt>>();

    let style_attr = stmt!(SubgraphAttributes::style("dotted".to_string()));

    let g = Graph::Graph {
        id: id!("RTTIClasses"),
        strict: true,
        stmts: [vec![nodes], vec![style_attr], edge_statements].concat(),
    };

    g.print(&mut PrinterContext::default())
}

fn map_node(node: &analysis::Node) -> Stmt {
    let id = escape(&node.id);
    let label = escape(&node.label);

    match node.node_type {
        NodeType::Class => stmt!(node!(
            id;
            attr!("label", label),
            attr!("shape", "box")
        )),
        NodeType::Namespace => {
            let children = node.children.iter()
                .map(|c| stmt!(map_node(c.1)))
                .collect::<Vec<Stmt>>();

            stmt!(Subgraph {
                id: Id::Escaped(id),
                stmts: children,
            })
        }
    }
}

fn escape<T: fmt::Display>(a: T) -> String {
    format!("\"{}\"", a).to_string()
}