# vim-plugin-metadata

Parse and analyze your vim plugins.

WARNING: This library is early alpha, still missing tons of functionality, and probably has serious
bugs. Use at your own risk.

## Usage

pip install it, point it at a file, get metadata:

```python
import vim_plugin_metadata

parser = vim_plugin_metadata.VimParser()
parser.parse_module("""
""
" Standalone header comment

""
" Does something cool.
func MyFunc() abort
  ...
endfunc
""")
```
```
VimModule(nodes=[StandaloneDocComment("Standalone header comment"), Function(name="MyFunc", doc="Does something cool.")])
```
