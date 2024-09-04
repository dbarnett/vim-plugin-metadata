use pyo3::prelude::*;

#[pymodule(name = "vim_plugin_metadata")]
mod py_vim_plugin_metadata {
    use super::*;
    use pyo3::exceptions::{PyException, PyIOError};
    use std::path::PathBuf;
    use vim_plugin_metadata;

    /// A representation of a single high-level grammar token of vim syntax,
    /// such as a comment or function.
    #[pyclass]
    #[derive(Clone, Debug, PartialEq)]
    pub enum VimNode {
        StandaloneDocComment {
            text: String,
        },
        Function {
            name: String,
            args: Vec<String>,
            modifiers: Vec<String>,
            doc: Option<String>,
        },
        Command {
            name: String,
            modifiers: Vec<String>,
            doc: Option<String>,
        },
        /// A defined "Flag" like the mechanism used in google/vim-maktaba.
        Flag {
            name: String,
            default_value_token: Option<String>,
            doc: Option<String>,
        },
    }

    #[pymethods]
    impl VimNode {
        pub fn __repr__(&self) -> String {
            match &self {
                Self::StandaloneDocComment { text } => {
                    format!("StandaloneDocComment({text:?})")
                }
                Self::Function {
                    name,
                    args,
                    modifiers,
                    doc,
                } => {
                    let mut args_str =
                        format!("name={name:?}, args={args:?}, modifiers={modifiers:?}");
                    if let Some(doc) = doc {
                        args_str.push_str(format!(", doc={doc:?}").as_str());
                    }
                    format!("Function({args_str})")
                }
                VimNode::Command {
                    name,
                    modifiers,
                    doc,
                } => {
                    let mut args_str = format!("name={name:?}, modifiers={modifiers:?}");
                    if let Some(doc) = doc {
                        args_str.push_str(format!(", doc={doc:?}").as_str());
                    }
                    format!("Command({args_str})")
                }
                Self::Flag {
                    name,
                    default_value_token,
                    doc,
                } => {
                    let mut args_str = format!("name={name:?}");
                    if let Some(default_value_token) = default_value_token {
                        args_str.push_str(
                            format!(", default_value_token={default_value_token:?}").as_str(),
                        );
                    }
                    if let Some(doc) = doc {
                        args_str.push_str(format!(", doc={doc:?}").as_str());
                    }
                    format!("Flag({args_str})")
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
                vim_plugin_metadata::VimNode::Function {
                    name,
                    args,
                    modifiers,
                    doc,
                } => Self::Function {
                    name,
                    args,
                    modifiers,
                    doc,
                },
                vim_plugin_metadata::VimNode::Command {
                    name,
                    modifiers,
                    doc,
                } => Self::Command {
                    name,
                    modifiers,
                    doc,
                },
                vim_plugin_metadata::VimNode::Flag {
                    name,
                    default_value_token,
                    doc,
                } => Self::Flag {
                    name,
                    default_value_token,
                    doc,
                },
            }
        }
    }

    /// An individual module (a.k.a. file) of vimscript code.
    #[pyclass]
    #[derive(Clone, Debug, PartialEq)]
    pub struct VimModule {
        pub path: Option<PathBuf>,
        pub doc: Option<String>,
        pub nodes: Vec<VimNode>,
    }

    #[pymethods]
    impl VimModule {
        #[getter]
        pub fn get_path(&self) -> Option<PathBuf> {
            self.path.as_ref().map(PathBuf::from)
        }

        #[getter]
        pub fn get_doc(&self) -> Option<String> {
            self.doc.to_owned()
        }

        #[getter]
        pub fn get_nodes(&self) -> Vec<VimNode> {
            self.nodes.clone()
        }

        pub fn __repr__(&self) -> String {
            format!("VimModule({:?}, ...)", self.path)
        }
    }

    impl From<vim_plugin_metadata::VimModule> for VimModule {
        fn from(module: vim_plugin_metadata::VimModule) -> Self {
            Self {
                path: module.path,
                doc: module.doc,
                nodes: module.nodes.into_iter().map(|n| n.into()).collect(),
            }
        }
    }

    /// An entire vim plugin with all the metadata parsed from its files.
    #[pyclass]
    #[derive(Clone, Debug, PartialEq)]
    pub struct VimPlugin {
        pub content: Vec<VimModule>,
    }

    #[pymethods]
    impl VimPlugin {
        #[getter]
        pub fn get_content(&self) -> Vec<VimModule> {
            self.content.clone()
        }

        pub fn __repr__(&self) -> String {
            format!(
                "VimPlugin([{}])",
                self.content
                    .iter()
                    .map(VimModule::__repr__)
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

        /// Parses all supported metadata from a single plugin at the given path.
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

        /// Parses and returns metadata for a single module (a.k.a. file) of vimscript code.
        pub fn parse_module_file(&mut self, path: &str) -> PyResult<VimModule> {
            let module = self
                .rust_parser
                .parse_module_file(path)
                .map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(module.into())
        }

        /// Parses and returns metadata for a single module (a.k.a. file) of vimscript code.
        pub fn parse_module_str(&mut self, code: &str) -> PyResult<VimModule> {
            let module = self
                .rust_parser
                .parse_module_str(code)
                .map_err(|err| PyException::new_err(format!("{err}")))?;
            Ok(module.into())
        }
    }
}
