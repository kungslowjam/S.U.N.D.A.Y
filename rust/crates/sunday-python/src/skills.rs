//! PyO3 bindings for skill manifests and verification.

use pyo3::prelude::*;

#[pyclass(name = "SkillManifest")]
pub struct PySkillManifest {
    inner: sunday_skills::SkillManifest,
}

#[pymethods]
impl PySkillManifest {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn version(&self) -> &str {
        &self.inner.version
    }

    #[getter]
    fn description(&self) -> &str {
        &self.inner.description
    }

    #[getter]
    fn author(&self) -> &str {
        &self.inner.author
    }

    #[getter]
    fn steps_count(&self) -> usize {
        self.inner.steps.len()
    }

    #[getter]
    fn required_capabilities(&self) -> Vec<String> {
        self.inner.required_capabilities.clone()
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    #[getter]
    fn depends(&self) -> Vec<String> {
        self.inner.depends.clone()
    }

    #[getter]
    fn user_invocable(&self) -> bool {
        self.inner.user_invocable
    }

    #[getter]
    fn disable_model_invocation(&self) -> bool {
        self.inner.disable_model_invocation
    }

    #[getter]
    fn markdown_content(&self) -> &str {
        &self.inner.markdown_content
    }

    #[getter]
    fn metadata_json(&self) -> String {
        serde_json::to_string(&self.inner.metadata).unwrap_or_else(|_| "{}".to_string())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&self.inner).unwrap_or_default()
    }

    fn manifest_bytes(&self) -> Vec<u8> {
        self.inner.manifest_bytes()
    }

    fn verify_signature(&self, public_key_hex: &str) -> bool {
        let key_bytes: Vec<u8> = (0..public_key_hex.len())
            .step_by(2)
            .filter_map(|i| u8::from_str_radix(&public_key_hex[i..i + 2], 16).ok())
            .collect();
        sunday_skills::verify_signature(&self.inner, &key_bytes)
    }
}

#[pyfunction]
pub fn load_skill(toml_str: &str) -> PyResult<PySkillManifest> {
    let manifest = sunday_skills::load_skill(toml_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    Ok(PySkillManifest { inner: manifest })
}

#[pyfunction]
pub fn parse_skill_markdown(raw: &str) -> PyResult<PySkillManifest> {
    let manifest = sunday_skills::parser::parse_skill_markdown(raw)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?;
    Ok(PySkillManifest { inner: manifest })
}
