use pyo3::prelude::*;

#[pymodule(name = "vim_plugin_metadata")]
mod py_vim_plugin_metadata {
    use super::*;
    use pyo3::exceptions::PyException;
    use vim_plugin_metadata;

    #[pyclass]
    #[derive(Clone, Debug, PartialEq)]
    pub enum VimNode {
        StandaloneDocComment { text: String },
        Function { name: String, doc: Option<String> },
    }

    #[pymethods]
    impl VimNode {
        pub fn __repr__(&self) -> String {
            match &self {
                Self::StandaloneDocComment { text } => {
                    format!("StandaloneDocComment({text:?})")
                }
                Self::Function { name, doc } => {
                    let mut args_str = format!("name={name:?}");
                    if let Some(doc) = doc {
                        args_str.push_str(format!(", doc={doc:?}").as_str());
                    }
                    format!("Function({args_str})")
                }
            }
        }
    }

    impl From<vim_plugin_metadata::VimNode> for VimNode {
        fn from(n: vim_plugin_metadata::VimNode) -> Self {
            match n {
                vim_plugin_metadata::VimNode::StandaloneDocComment(text) => {
                    Self::StandaloneDocComment { text }
                }
                vim_plugin_metadata::VimNode::Function { name, doc } => {
                    Self::Function { name, doc }
                }
            }
        }
    }

    #[pyclass]
    #[derive(Default)]
    pub struct VimParser {
        rust_parser: vim_plugin_metadata::VimParser,
    }

    #[pymethods]
    impl VimParser {
        #[new]
        pub fn new() -> PyResult<Self> {
            let rust_parser = vim_plugin_metadata::VimParser::new()
                .map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(Self { rust_parser })
        }

        pub fn parse_module(&mut self, code: &str) -> PyResult<Vec<VimNode>> {
            let module = self
                .rust_parser
                .parse_module(code)
                .map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(module.into_iter().map(|n| n.into()).collect())
        }
    }
}
