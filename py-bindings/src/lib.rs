use pyo3::prelude::*;

#[pymodule(name = "vim_plugin_metadata")]
mod py_vim_plugin_metadata {
    use super::*;
    use pyo3::exceptions::{PyException, PyIOError};
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
    #[derive(Clone, Debug, PartialEq)]
    pub struct VimPluginSection {
        pub name: String,
        pub nodes: Vec<VimNode>,
    }

    #[pymethods]
    impl VimPluginSection {
        #[getter]
        pub fn get_name(&self) -> &str {
            self.name.as_ref()
        }

        #[getter]
        pub fn get_nodes(&self) -> Vec<VimNode> {
            self.nodes.clone()
        }

        pub fn __repr__(&self) -> String {
            format!("VimPluginSection({:?}, ...)", self.name)
        }
    }

    impl From<vim_plugin_metadata::VimPluginSection> for VimPluginSection {
        fn from(section: vim_plugin_metadata::VimPluginSection) -> Self {
            Self {
                name: section.name,
                nodes: section.nodes.into_iter().map(|n| n.into()).collect(),
            }
        }
    }

    #[pyclass]
    #[derive(Clone, Debug, PartialEq)]
    pub struct VimPlugin {
        pub content: Vec<VimPluginSection>,
    }

    #[pymethods]
    impl VimPlugin {
        #[getter]
        pub fn get_content(&self) -> Vec<VimPluginSection> {
            self.content.clone()
        }

        pub fn __repr__(&self) -> String {
            format!(
                "VimPlugin([{}])",
                self.content
                    .iter()
                    .map(VimPluginSection::__repr__)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    impl From<vim_plugin_metadata::VimPlugin> for VimPlugin {
        fn from(plugin: vim_plugin_metadata::VimPlugin) -> Self {
            Self {
                content: plugin
                    .content
                    .into_iter()
                    .map(|section| section.into())
                    .collect(),
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

        pub fn parse_plugin_dir(&mut self, path: &str) -> PyResult<VimPlugin> {
            let plugin = self
                .rust_parser
                .parse_plugin_dir(path)
                .map_err(|err| match err {
                    vim_plugin_metadata::Error::IOError(io_error) => {
                        PyIOError::new_err(format!("{io_error}"))
                    }
                    _ => PyException::new_err(format!("{err}")),
                })?;
            Ok(plugin.into())
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
