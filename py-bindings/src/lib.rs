use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use vim_plugin_metadata::{VimModule, VimNode, VimParser};

#[pymodule(name = "vim_plugin_metadata")]
mod py_vim_plugin_metadata {
    use super::*;

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

    impl From<super::VimNode> for VimNode {
        fn from(n: super::VimNode) -> VimNode {
            match n {
                super::VimNode::StandaloneDocComment(text) => {
                    VimNode::StandaloneDocComment { text }
                }
                super::VimNode::Function { name, doc } => VimNode::Function { name, doc },
            }
        }
    }

    #[pyclass]
    #[derive(Debug)]
    pub struct VimModule {
        pub nodes: Vec<VimNode>,
    }

    #[pymethods]
    impl VimModule {
        #[getter]
        pub fn get_nodes(&self) -> Vec<VimNode> {
            self.nodes.clone()
        }

        pub fn __repr__(&self) -> String {
            format!(
                "VimModule(nodes=[{}])",
                self.nodes
                    .iter()
                    .map(|n| n.__repr__())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    impl From<super::VimModule> for VimModule {
        fn from(m: super::VimModule) -> VimModule {
            VimModule {
                nodes: m.nodes.into_iter().map(|n| n.into()).collect(),
            }
        }
    }

    #[pyclass]
    #[derive(Default)]
    pub struct VimParser {
        rust_parser: super::VimParser,
    }

    #[pymethods]
    impl VimParser {
        #[new]
        pub fn new() -> PyResult<Self> {
            let rust_parser =
                super::VimParser::new().map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(Self { rust_parser })
        }

        pub fn parse_module(&mut self, code: &str) -> PyResult<VimModule> {
            let module = self
                .rust_parser
                .parse_module(code)
                .map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(module.into())
        }
    }
}
