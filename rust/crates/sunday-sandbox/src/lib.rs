use wasmtime::*;
use wasi_common::sync::WasiCtxBuilder;
use std::sync::Arc;
use serde_json::Value;

/// Secure WebAssembly Sandbox for untrusted code execution.
pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.consume_fuel(true); // Enable CPU limiting
        config.wasm_component_model(true);
        
        Self {
            engine: Engine::new(&config).expect("Failed to create Wasmtime engine"),
        }
    }

    /// Run a Wasm module with restricted resources.
    pub fn run_module(&self, wasm_bytes: &[u8], fuel_limit: u64) -> anyhow::Result<String> {
        let module = Module::from_binary(&self.engine, wasm_bytes)?;
        
        let wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();

        let mut store = Store::new(&self.engine, wasi);
        store.set_fuel(fuel_limit)?;

        let linker = Linker::new(&self.engine);
        wasi_common::sync::add_to_linker(&linker, |s| s)?;

        let instance = linker.instantiate(&mut store, &module)?;
        let start = instance.get_typed_func::<(), ()>(&mut store, "_start")?;

        start.call(&mut store, ())?;

        Ok("Execution completed successfully".to_string())
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyclass(name = "NativeSandbox")]
pub struct NativeSandbox {
    inner: Sandbox,
}

#[cfg(feature = "python")]
#[pymethods]
impl NativeSandbox {
    #[new]
    fn new() -> Self {
        Self { inner: Sandbox::new() }
    }

    /// Run a WASI-compliant Wasm binary with a specified fuel (CPU) limit.
    #[pyo3(signature = (wasm_bytes, fuel_limit=1000000))]
    fn run_wasm(&self, wasm_bytes: Vec<u8>, fuel_limit: u64) -> PyResult<String> {
        self.inner.run_module(&wasm_bytes, fuel_limit)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn sunday_sandbox(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<NativeSandbox>()?;
    Ok(())
}
