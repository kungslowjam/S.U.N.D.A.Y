//! PyO3 bindings for core types.

use pyo3::prelude::*;
use std::collections::HashMap;

#[pyclass(name = "Message")]
#[derive(Clone)]
pub struct PyMessage {
    #[pyo3(get, set)]
    pub role: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub name: Option<String>,
    #[pyo3(get, set)]
    pub tool_call_id: Option<String>,
}

#[pymethods]
impl PyMessage {
    #[new]
    fn new(role: String, content: String) -> Self {
        Self {
            role,
            content,
            name: None,
            tool_call_id: None,
        }
    }

    fn __repr__(&self) -> String {
        format!("Message(role='{}', content='{}')", self.role, &self.content[..self.content.len().min(50)])
    }
}

impl PyMessage {
    pub fn to_core(&self) -> sunday_core::Message {
        let role = match self.role.as_str() {
            "system" => sunday_core::Role::System,
            "assistant" => sunday_core::Role::Assistant,
            "tool" => sunday_core::Role::Tool,
            _ => sunday_core::Role::User,
        };
        sunday_core::Message {
            role,
            content: self.content.clone(),
            name: self.name.clone(),
            tool_calls: None,
            tool_call_id: self.tool_call_id.clone(),
            metadata: HashMap::new(),
        }
    }
}

#[pyclass(name = "ToolResult")]
#[derive(Clone)]
pub struct PyToolResult {
    #[pyo3(get)]
    pub tool_name: String,
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub success: bool,
}

#[pymethods]
impl PyToolResult {
    #[new]
    fn new(tool_name: String, content: String, success: bool) -> Self {
        Self { tool_name, content, success }
    }

    fn __repr__(&self) -> String {
        format!("ToolResult(tool='{}', success={})", self.tool_name, self.success)
    }
}

#[pyclass(name = "ToolCall")]
#[derive(Clone)]
pub struct PyToolCall {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub arguments: String,
}

#[pymethods]
impl PyToolCall {
    #[new]
    fn new(id: String, name: String, arguments: String) -> Self {
        Self { id, name, arguments }
    }
}

#[pyclass(name = "Config")]
pub struct PyConfig {
    pub inner: sunday_core::JarvisConfig,
}

#[pymethods]
impl PyConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: sunday_core::JarvisConfig::default(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Config(engine={}, model={})",
            self.inner.engine.default, self.inner.intelligence.default_model
        )
    }

    #[getter]
    fn engine_default(&self) -> String {
        self.inner.engine.default.clone()
    }

    #[getter]
    fn model_default(&self) -> String {
        self.inner.intelligence.default_model.clone()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        let json_mod = py.import("json")?;
        Ok(json_mod.call_method1("loads", (json_str,))?.unbind())
    }
}

#[pyclass(name = "Event")]
#[derive(Clone)]
pub struct PyEvent {
    #[pyo3(get)]
    pub event_type: String,
    #[pyo3(get)]
    pub timestamp: f64,
    pub data: serde_json::Value,
}

#[pymethods]
impl PyEvent {
    #[getter]
    fn data(&self, py: Python<'_>) -> PyResult<PyObject> {
        let json_str = self.data.to_string();
        let json_mod = py.import("json")?;
        Ok(json_mod.call_method1("loads", (json_str,))?.unbind())
    }

    fn __repr__(&self) -> String {
        format!("Event(type='{}', ts={:.2})", self.event_type, self.timestamp)
    }
}

#[pyclass(name = "EventBus")]
pub struct PyEventBus {
    pub inner: std::sync::Arc<sunday_core::EventBus>,
}

#[pymethods]
impl PyEventBus {
    #[new]
    #[pyo3(signature = (record_history=None))]
    fn new(record_history: Option<bool>) -> Self {
        Self {
            inner: sunday_core::events::GLOBAL_BUS.clone(),
        }
    }

    fn subscribe(&self, _event_type: String, callback: PyObject) {
        let inner = self.inner.clone();
        // Wrap the Python callback in a Rust closure
        let rust_callback = move |event: std::sync::Arc<sunday_core::Event>| {
            Python::with_gil(|py| {
                let py_event = PyEvent {
                    event_type: event.event_type.to_string(),
                    timestamp: event.timestamp,
                    data: event.data.clone(),
                };
                let _ = callback.call1(py, (py_event,));
            });
        };
        let _guard = crate::RUNTIME.enter();
        inner.subscribe_callback(Box::new(rust_callback));
    }

    #[pyo3(signature = (event_type, data=None))]
    fn publish(&self, py: Python<'_>, event_type: String, data: Option<Bound<'_, pyo3::types::PyDict>>) -> PyResult<PyEvent> {
        let data_val = if let Some(d) = data {
            let json_str = py.import("json")?.call_method1("dumps", (d,))?.extract::<String>()?;
            serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };

        use std::str::FromStr;
        let et = sunday_core::EventType::from_str(&event_type)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;

        let event = self.inner.publish(et, data_val);
        Ok(PyEvent {
            event_type: event.event_type.to_string(),
            timestamp: event.timestamp,
            data: event.data.clone(),
        })
    }

    fn history(&self) -> Vec<PyEvent> {
        self.inner.history().into_iter().map(|e| PyEvent {
            event_type: e.event_type.to_string(),
            timestamp: e.timestamp,
            data: e.data.clone(),
        }).collect()
    }

    fn clear_history(&self) {
        self.inner.clear_history();
    }

    fn history_len(&self) -> usize {
        self.inner.history().len()
    }
}

#[pyclass(name = "ModelSpec")]
#[derive(Clone)]
pub struct PyModelSpec {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub params_b: f64,
    #[pyo3(get, set)]
    pub context_length: usize,
}

#[pymethods]
impl PyModelSpec {
    #[new]
    fn new(name: String, params_b: f64, context_length: usize) -> Self {
        Self { name, params_b, context_length }
    }
}

#[pyclass(name = "RoutingContext")]
#[derive(Clone)]
pub struct PyRoutingContext {
    #[pyo3(get, set)]
    pub query: String,
    #[pyo3(get, set)]
    pub query_class: String,
}

#[pymethods]
impl PyRoutingContext {
    #[new]
    fn new(query: String) -> Self {
        Self { query, query_class: "general".into() }
    }
}

#[pyclass(name = "AgentContext")]
pub struct PyAgentContext {
    #[pyo3(get, set)]
    pub session_id: String,
}

#[pymethods]
impl PyAgentContext {
    #[new]
    fn new(session_id: String) -> Self {
        Self { session_id }
    }
}

#[pyclass(name = "AgentResult")]
#[derive(Clone)]
pub struct PyAgentResult {
    #[pyo3(get)]
    pub content: String,
    #[pyo3(get)]
    pub turns: usize,
}

#[pymethods]
impl PyAgentResult {
    fn __repr__(&self) -> String {
        format!("AgentResult(turns={}, content='{}')", self.turns, &self.content[..self.content.len().min(50)])
    }
}

#[pyclass(name = "Tokenizer")]
pub struct PyTokenizer;

#[pymethods]
impl PyTokenizer {
    #[staticmethod]
    fn load_from_file(name: String, path: String) -> PyResult<()> {
        let p = std::path::Path::new(&path);
        sunday_core::tokenizer::TOKENIZER.load_from_file(&name, p)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    #[staticmethod]
    fn count_tokens(name: String, text: String) -> usize {
        sunday_core::tokenizer::TOKENIZER.count_tokens(&name, &text)
    }

    #[staticmethod]
    fn count_tokens_batch(name: String, texts: Vec<String>) -> Vec<usize> {
        use rayon::prelude::*;
        texts.into_par_iter()
            .map(|text| sunday_core::tokenizer::TOKENIZER.count_tokens(&name, &text))
            .collect()
    }
}
