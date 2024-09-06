# vim-plugin-metadata

Parse and analyze your vim plugins.

WARNING: This library is in early development, still missing functionality, and probably has plenty
of bugs. Use at your own risk.

## Usage

pip install it, point it at a file, get metadata:

```python
import vim_plugin_metadata

parser = vim_plugin_metadata.VimParser()
parser.parse_plugin_dir(".vim/plugged/someplugin")
```
```
VimPlugin([VimModule("plugin/somefile.vim", …), VimModule("autoload/someplugin.vim", …)])
```

```python
parser.parse_module_str("""
""
" File header comment

""
" Does something cool.
func MyFunc() abort
  …
endfunc
""")
```
```
VimModule(doc="File header comment", nodes=[Function(name="MyFunc", args=[], modifiers=["abort"], doc="Does something cool.")])
```
