use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use crate::RUNTIME;
use sunday_workflow::{WorkflowGraph, WorkflowEngine, WorkflowNode, WorkflowEdge, NodeType, WorkflowSystem};

#[pyclass(name = "WorkflowGraph")]
pub struct PyWorkflowGraph {
    pub(crate) inner: WorkflowGraph,
}

#[pymethods]
impl PyWorkflowGraph {
    #[new]
    #[pyo3(signature = (name=""))]
    fn new(name: &str) -> Self {
        Self {
            inner: WorkflowGraph::new(name.to_string()),
        }
    }

    #[pyo3(signature = (id, node_type, agent=None, tools=None, config=None, condition_expr=None, transform_expr=None, max_iterations=None))]
    fn add_node(
        &mut self,
        id: String,
        node_type: &str,
        agent: Option<String>,
        tools: Option<Vec<String>>,
        config: Option<PyObject>,
        condition_expr: Option<String>,
        transform_expr: Option<String>,
        max_iterations: Option<u32>,
    ) -> PyResult<()> {
        let nt = match node_type {
            "agent" => NodeType::Agent,
            "tool" => NodeType::Tool,
            "condition" => NodeType::Condition,
            "transform" => NodeType::Transform,
            "loop" => NodeType::Loop,
            _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Unknown node type: {}", node_type))),
        };
        
        let config_map = if let Some(c) = config {
            Python::with_gil(|py| {
                let json_str: String = py.import("json")?.call_method1("dumps", (c,))?.extract()?;
                serde_json::from_str(&json_str)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
            })?
        } else {
            HashMap::new()
        };

        let node = WorkflowNode {
            id,
            node_type: nt,
            agent,
            tools,
            config: config_map,
            condition_expr,
            transform_expr,
            max_iterations,
        };
        
        self.inner.add_node(node)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[pyo3(signature = (source, target, condition=None))]
    fn add_edge(&mut self, source: String, target: String, condition: Option<String>) -> PyResult<()> {
        let edge = WorkflowEdge {
            source,
            target,
            condition,
        };
        self.inner.add_edge(edge)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn validate(&self) -> PyResult<()> {
        self.inner.validate()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn topological_sort(&self) -> PyResult<Vec<String>> {
        self.inner.topological_sort()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

/// Bridge for Python system to implement WorkflowSystem trait
struct PyWorkflowSystem {
    py_system: PyObject,
}

impl WorkflowSystem for PyWorkflowSystem {
    fn execute_agent(&self, input: &str, agent: Option<&str>, tools: Option<&[String]>) -> anyhow::Result<String> {
        Python::with_gil(|py| {
            let res = self.py_system.call_method1(py, "ask", (input, agent, tools))?;
            let content: String = res.bind(py).get_item("content")?.extract()?;
            Ok(content)
        })
    }

    fn execute_tool(&self, name: &str, args: &str) -> anyhow::Result<String> {
        Python::with_gil(|py| {
            let res = self.py_system.call_method1(py, "execute_tool", (name, args))?;
            let content: String = res.bind(py).extract()?;
            Ok(content)
        })
    }
}

#[pyclass(name = "WorkflowEngine")]
pub struct PyWorkflowEngine {
    inner: WorkflowEngine,
}

#[pymethods]
impl PyWorkflowEngine {
    #[new]
    #[pyo3(signature = (max_parallel=4))]
    fn new(max_parallel: usize) -> Self {
        Self {
            inner: WorkflowEngine::new(max_parallel),
        }
    }

    fn run(&self, graph: &PyWorkflowGraph, py_system: PyObject, initial_input: String) -> PyResult<String> {
        let system = Arc::new(PyWorkflowSystem { py_system });
        let engine = &self.inner;
        let inner_graph = &graph.inner;
        
        let result = RUNTIME.block_on(async {
            engine.run(inner_graph, system, initial_input).await
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        
        Ok(serde_json::to_string(&result).unwrap_or_default())
    }
}
