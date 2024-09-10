use pyo3::prelude::*;

/// A library to parse and analyze your vim plugins.
///
/// The main use case is to instantiate a VimParser, configure it, and point
/// it to a plugin dir or file to parse.
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
            doc: String,
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
        Variable {
            name: String,
            init_value_token: String,
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
                Self::StandaloneDocComment { doc } => {
                    format!("StandaloneDocComment(doc={doc:?})")
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
                Self::Command {
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
                Self::Variable {
                    name,
                    init_value_token,
                    doc,
                } => {
                    let mut args_str =
                        format!("name={name:?}, init_value_token={init_value_token:?}");
                    if let Some(doc) = doc {
                        args_str.push_str(format!(", doc={doc:?}").as_str());
                    }
                    format!("Flag({args_str})")
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
                vim_plugin_metadata::VimNode::StandaloneDocComment { doc } => {
                    Self::StandaloneDocComment { doc }
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
                vim_plugin_metadata::VimNode::Variable {
                    name,
                    init_value_token,
                    doc,
                } => Self::Variable {
                    name,
                    init_value_token,
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
        #[pyo3(get)]
        pub doc: Option<String>,
        #[pyo3(get)]
        pub nodes: Vec<VimNode>,
    }

    #[pymethods]
    impl VimModule {
        #[getter]
        pub fn get_path(&self) -> Result<PyObject, PyErr> {
            Python::with_gil(|py| match &self.path {
                None => Ok(py.None()),
                Some(path) => {
                    let pathlib = PyModule::import_bound(py, "pathlib")?;
                    pathlib.getattr("Path")?.call1((path,))?.extract()
                }
            })
        }

        pub fn __repr__(&self) -> String {
            let mut args_strs = Vec::with_capacity(3);
            if let Some(path) = &self.path {
                args_strs.push(format!("path={:?}", path.to_str().unwrap()));
            }
            if let Some(doc) = &self.doc {
                args_strs.push(format!(
                    "doc={:?}",
                    unicode_ellipsis::truncate_str(doc, 100)
                ));
            }
            args_strs.push(format!(
                "nodes=[{}]",
                match &self.nodes[..] {
                    [a, b, c, _, ..] =>
                        format!("{}, {}, {}, …", a.__repr__(), b.__repr__(), c.__repr__()),
                    nodes => nodes
                        .iter()
                        .map(VimNode::__repr__)
                        .collect::<Vec<_>>()
                        .join(", "),
                }
            ));
            format!("VimModule({})", args_strs.join(", "))
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
        #[pyo3(get)]
        pub content: Vec<VimModule>,
    }

    #[pymethods]
    impl VimPlugin {
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

    /// The main entry point for parsing plugins.
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
        pub fn parse_plugin_dir(&mut self, path: PathBuf) -> PyResult<VimPlugin> {
            let plugin = self
                .rust_parser
                .parse_plugin_dir(&path)
                .map_err(|err| match err {
                    vim_plugin_metadata::Error::IOError(io_error) => {
                        PyIOError::new_err(format!("{io_error}"))
                    }
                    _ => PyException::new_err(format!("{err}")),
                })?;
            Ok(plugin.into())
        }

        /// Parses and returns metadata for a single module (a.k.a. file) of vimscript code.
        pub fn parse_module_file(&mut self, path: PathBuf) -> PyResult<VimModule> {
            let module = self
                .rust_parser
                .parse_module_file(&path)
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
