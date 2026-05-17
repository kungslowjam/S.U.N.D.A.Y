use std::collections::HashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use crate::types::{WorkflowNode, WorkflowEdge};
use anyhow::{Result, anyhow};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

pub struct WorkflowGraph {
    pub name: String,
    pub nodes: HashMap<String, (NodeIndex, WorkflowNode)>,
    edges: Vec<WorkflowEdge>,
    graph: DiGraph<String, ()>,
}

impl WorkflowGraph {
    pub fn new(name: String) -> Self {
        Self {
            name,
            nodes: HashMap::new(),
            edges: Vec::new(),
            graph: DiGraph::new(),
        }
    }

    pub fn from_raw(name: String, raw: RawGraph) -> Result<Self> {
        let mut graph = Self::new(name);
        for node in raw.nodes {
            graph.add_node(node)?;
        }
        for edge in raw.edges {
            graph.add_edge(edge)?;
        }
        Ok(graph)
    }

    pub fn add_node(&mut self, node: WorkflowNode) -> Result<()> {
        if self.nodes.contains_key(&node.id) {
            return Err(anyhow!("Duplicate node id: {}", node.id));
        }
        let idx = self.graph.add_node(node.id.clone());
        self.nodes.insert(node.id.clone(), (idx, node));
        Ok(())
    }

    pub fn add_edge(&mut self, edge: WorkflowEdge) -> Result<()> {
        let (source_idx, _) = self.nodes.get(&edge.source)
            .ok_or_else(|| anyhow!("Source node '{}' not found", edge.source))?;
        let (target_idx, _) = self.nodes.get(&edge.target)
            .ok_or_else(|| anyhow!("Target node '{}' not found", edge.target))?;
        
        self.graph.add_edge(*source_idx, *target_idx, ());
        self.edges.push(edge);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&WorkflowNode> {
        self.nodes.get(id).map(|(_, n)| n)
    }

    pub fn validate(&self) -> Result<()> {
        toposort(&self.graph, None)
            .map_err(|e| anyhow!("Cycle detected: {:?}", e))?;
        Ok(())
    }

    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let nodes = toposort(&self.graph, None)
            .map_err(|e| anyhow!("Cycle detected: {:?}", e))?;
        
        Ok(nodes.into_iter()
            .map(|idx| self.graph[idx].clone())
            .collect())
    }

    pub fn execution_stages(&self) -> Vec<Vec<String>> {
        // Simple stage grouping: Kahn's inspired
        let mut in_degree: HashMap<NodeIndex, usize> = HashMap::new();
        for node in self.graph.node_indices() {
            in_degree.insert(node, self.graph.neighbors_directed(node, petgraph::Direction::Incoming).count());
        }

        let mut stages = Vec::new();
        let mut ready: Vec<NodeIndex> = in_degree.iter()
            .filter(|&(_, &deg)| deg == 0)
            .map(|(&idx, _)| idx)
            .collect();

        while !ready.is_empty() {
            let mut current_stage = Vec::new();
            let mut next_ready = Vec::new();

            for node_idx in ready {
                current_stage.push(self.graph[node_idx].clone());
                for neighbor in self.graph.neighbors(node_idx) {
                    let deg = in_degree.get_mut(&neighbor).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_ready.push(neighbor);
                    }
                }
            }
            stages.push(current_stage);
            ready = next_ready;
        }

        stages
    }

    pub fn predecessors(&self, node_id: &str) -> Vec<String> {
        if let Some((idx, _)) = self.nodes.get(node_id) {
            self.graph.neighbors_directed(*idx, petgraph::Direction::Incoming)
                .map(|i| self.graph[i].clone())
                .collect()
        } else {
            Vec::new()
        }
    }
}
