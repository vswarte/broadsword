use std::collections::HashMap;

use crate::symbol::TypeInfoSymbol;

#[derive(Debug)]
pub(crate) struct Node {
    pub node_type: NodeType,
    pub id: String,
    pub label:  String,
    pub children: HashMap<String, Box<Node>>,
}

#[derive(Debug)]
pub(crate) enum NodeType {
    Namespace,
    Class,
}

/// Takes a list of TypeInfoSymbols and outputs them as a tree based on the namespacing.
pub(crate) fn map_into_tree(symbols: Vec<(String, TypeInfoSymbol)>) -> Node {
    let mut children = HashMap::default();

    for (ibo, symbol) in symbols {
        let mut current_namespace = &mut children;
        for namespace in symbol.namespaces.iter() {
            // Insert the namespace if it doesn't exist yet
            if !current_namespace.contains_key(namespace) {
                current_namespace.insert(namespace.clone(), Box::new(Node {
                    node_type: NodeType::Namespace,
                    id: namespace.clone(),
                    label: namespace.clone(),
                    children: HashMap::default(),
                }));
            }

            // Move to the new namespace
            current_namespace = &mut current_namespace.get_mut(namespace).unwrap().children;
        }

        current_namespace.insert(symbol.name.clone(), Box::new(Node {
            node_type: NodeType::Class,
            id: format!("{}", ibo).to_string(),
            label: format!("{} ({})", symbol.name.clone(), ibo).to_string(),
            children: HashMap::default(),
        }));
    }

    Node {
        node_type: NodeType::Namespace,
        id: String::from("root"),
        label: String::from("Root Namespace"),
        children
    }
}