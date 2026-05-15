//! PyO3 bindings for tool types.

use sunday_tools::traits::BaseTool;
use pyo3::prelude::*;
use std::sync::Arc;

#[pyclass(name = "ToolExecutor")]
pub struct PyToolExecutor {
    pub inner: Arc<sunday_tools::ToolExecutor>,
}

#[pymethods]
impl PyToolExecutor {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(sunday_tools::ToolExecutor::new(None, None, None)),
        }
    }

    fn list_tools(&self) -> Vec<String> {
        self.inner.list_tools()
    }

    fn execute(&self, tool_name: &str, params_json: &str) -> PyResult<String> {
        let params: serde_json::Value = serde_json::from_str(params_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let result = self
            .inner
            .execute(tool_name, &params, None, None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(serde_json::to_string(&result).unwrap_or_default())
    }
}

#[pyclass(name = "CalculatorTool")]
pub struct PyCalculatorTool;

#[pymethods]
impl PyCalculatorTool {
    #[new]
    fn new() -> Self {
        Self
    }

    fn execute(&self, expression: &str) -> PyResult<String> {
        let tool = sunday_tools::builtin::calculator::CalculatorTool;
        let params = serde_json::json!({"expression": expression});
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "ThinkTool")]
pub struct PyThinkTool;

#[pymethods]
impl PyThinkTool {
    #[new]
    fn new() -> Self {
        Self
    }

    fn execute(&self, thought: &str) -> PyResult<String> {
        let tool = sunday_tools::builtin::think::ThinkTool;
        let params = serde_json::json!({"thought": thought});
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "FileReadTool")]
pub struct PyFileReadTool;

#[pymethods]
impl PyFileReadTool {
    #[new]
    fn new() -> Self {
        Self
    }

    fn execute(&self, path: &str) -> PyResult<String> {
        let tool = sunday_tools::builtin::file_tools::FileReadTool;
        let params = serde_json::json!({"path": path});
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "FileWriteTool")]
pub struct PyFileWriteTool;

#[pymethods]
impl PyFileWriteTool {
    #[new]
    fn new() -> Self {
        Self
    }

    fn execute(&self, path: &str, content: &str) -> PyResult<String> {
        let tool = sunday_tools::builtin::file_tools::FileWriteTool;
        let params = serde_json::json!({"path": path, "content": content});
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "ShellExecTool")]
pub struct PyShellExecTool;

#[pymethods]
impl PyShellExecTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (command, cwd=None))]
    fn execute(&self, command: &str, cwd: Option<&str>) -> PyResult<String> {
        let tool = sunday_tools::builtin::shell::ShellExecTool;
        let mut params = serde_json::json!({"command": command});
        if let Some(cwd) = cwd {
            params["cwd"] = serde_json::Value::String(cwd.to_string());
        }
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "HttpRequestTool")]
pub struct PyHttpRequestTool;

#[pymethods]
impl PyHttpRequestTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (url, method="GET", body=None))]
    fn execute(&self, url: &str, method: &str, body: Option<&str>) -> PyResult<String> {
        let tool = sunday_tools::builtin::http_tools::HttpRequestTool;
        let mut params = serde_json::json!({"url": url, "method": method});
        if let Some(body) = body {
            params["body"] = serde_json::Value::String(body.to_string());
        }
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "GitStatusTool")]
pub struct PyGitStatusTool;

#[pymethods]
impl PyGitStatusTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (cwd=None))]
    fn execute(&self, cwd: Option<&str>) -> PyResult<String> {
        let tool = sunday_tools::builtin::git_tools::GitStatusTool;
        let mut params = serde_json::json!({});
        if let Some(cwd) = cwd {
            params["cwd"] = serde_json::Value::String(cwd.to_string());
        }
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "GitDiffTool")]
pub struct PyGitDiffTool;

#[pymethods]
impl PyGitDiffTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (cwd=None))]
    fn execute(&self, cwd: Option<&str>) -> PyResult<String> {
        let tool = sunday_tools::builtin::git_tools::GitDiffTool;
        let mut params = serde_json::json!({});
        if let Some(cwd) = cwd {
            params["cwd"] = serde_json::Value::String(cwd.to_string());
        }
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "GitLogTool")]
pub struct PyGitLogTool;

#[pymethods]
impl PyGitLogTool {
    #[new]
    fn new() -> Self {
        Self
    }

    #[pyo3(signature = (cwd=None, count=None))]
    fn execute(&self, cwd: Option<&str>, count: Option<u32>) -> PyResult<String> {
        let tool = sunday_tools::builtin::git_tools::GitLogTool;
        let mut params = serde_json::json!({});
        if let Some(cwd) = cwd {
            params["cwd"] = serde_json::Value::String(cwd.to_string());
        }
        if let Some(count) = count {
            params["count"] = serde_json::Value::Number(count.into());
        }
        let result = tool
            .execute(&params)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(result.content)
    }
}

#[pyclass(name = "AXTreeProcessor")]
pub struct PyAXTreeProcessor {
    pub inner: sunday_tools::browser_ax::AXTreeProcessor,
}

#[pymethods]
impl PyAXTreeProcessor {
    #[new]
    #[pyo3(signature = (max_depth=10, filter_unimportant=true))]
    fn new(max_depth: usize, filter_unimportant: bool) -> Self {
        Self {
            inner: sunday_tools::browser_ax::AXTreeProcessor::new(max_depth, filter_unimportant),
        }
    }

    fn process_json(&self, ax_tree_json: &str) -> PyResult<String> {
        let root: serde_json::Value = serde_json::from_str(ax_tree_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(self.inner.process(&root))
    }

    #[pyo3(signature = (ax_tree_json, shm_name="sunday_ax_tree"))]
    fn process_to_shm(&self, ax_tree_json: &str, shm_name: &str) -> PyResult<String> {
        let root: serde_json::Value = serde_json::from_str(ax_tree_json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.inner.process_to_shm(&root, shm_name)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn process_batch(&self, ax_tree_jsons: Vec<String>) -> PyResult<Vec<String>> {
        use rayon::prelude::*;
        ax_tree_jsons
            .into_par_iter()
            .map(|json_str| {
                let root: serde_json::Value = serde_json::from_str(&json_str)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
                Ok(self.inner.process(&root))
            })
            .collect()
    }
}

#[pyclass(name = "NativeBrowser")]
pub struct PyNativeBrowser {
    inner: Arc<sunday_tools::browser_native::NativeBrowserSession>,
}

#[pymethods]
impl PyNativeBrowser {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(sunday_tools::browser_native::NativeBrowserSession::new()),
        }
    }

    #[pyo3(signature = (url, headless=false))]
    fn goto(&self, url: String, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let page = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            page.goto(url).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok("Navigation successful".to_string())
        })
    }

    #[pyo3(signature = (selector, headless=false))]
    fn click(&self, selector: String, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let page = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            page.find_element(&selector).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
                .click().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok("Click successful".to_string())
        })
    }

    #[pyo3(signature = (selector, text, headless=false))]
    fn fill(&self, selector: String, text: String, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let page = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            page.find_element(&selector).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?
                .type_str(text).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok("Typing successful".to_string())
        })
    }

    #[pyo3(signature = (headless=false))]
    fn get_ax_tree(&self, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            let tree = inner.get_ax_tree().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Ok(serde_json::to_string(&tree).unwrap_or_default())
        })
    }

    #[pyo3(signature = (headless=false))]
    fn content(&self, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let content = inner.get_content().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(content)
        })
    }

    #[pyo3(signature = (key, headless=false))]
    fn press(&self, key: String, headless: bool) -> PyResult<()> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            inner.press_key(&key).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    #[pyo3(signature = (x, y, headless=false))]
    fn scroll(&self, x: i32, y: i32, headless: bool) -> PyResult<()> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            inner.scroll(x, y).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(())
        })
    }

    #[pyo3(signature = (headless=false))]
    fn screenshot(&self, headless: bool) -> PyResult<String> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            let b64 = inner.capture_screenshot_base64().await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            Ok(b64)
        })
    }

    #[pyo3(signature = (buffer_name="sunday_browser_shot", headless=false))]
    fn capture_screenshot_shared(&self, buffer_name: &str, headless: bool) -> PyResult<usize> {
        let inner = self.inner.clone();
        crate::RUNTIME.block_on(async move {
            let _ = inner.ensure_page(headless).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            let size = inner.capture_screenshot_shared(&buffer_name).await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
            
            Ok(size)
        })
    }

    fn close(&self) -> PyResult<()> {
        Ok(())
    }
}

#[pyclass(name = "NativeMiner")]
pub struct PyNativeMiner {
    inner: sunday_mining::DOMMiner,
}

#[pymethods]
impl PyNativeMiner {
    #[new]
    fn new() -> Self {
        Self {
            inner: sunday_mining::DOMMiner::new(),
        }
    }

    fn mine_html(&self, html: &str) -> PyResult<String> {
        let nodes = self.inner.extract_tree(html);
        Ok(self.inner.format_for_llm(&nodes))
    }
}
