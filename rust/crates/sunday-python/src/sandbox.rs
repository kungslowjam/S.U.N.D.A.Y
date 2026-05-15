use pyo3::prelude::*;
use sunday_sandbox::NativeSandbox as CoreNativeSandbox;

#[pyclass(name = "NativeSandbox")]
pub struct PyNativeSandbox {
    pub inner: CoreNativeSandbox,
}

#[pymethods]
impl PyNativeSandbox {
    #[new]
    fn new() -> Self {
        Self {
            inner: CoreNativeSandbox::new(),
        }
    }

    /// Run a WASI-compliant Wasm binary with a specified fuel (CPU) limit.
    #[pyo3(signature = (wasm_bytes, fuel_limit=1000000))]
    fn run_wasm(&self, wasm_bytes: Vec<u8>, fuel_limit: u64) -> PyResult<String> {
        self.inner.run_wasm(wasm_bytes, fuel_limit)
    }
}
