from dataclasses import dataclass
from typing import List, Optional

class VimParser:
    def __init__(self): ...
    def parse_plugin_dir(self, path: str) -> VimPlugin: ...
    def parse_module(self, code: str) -> List[VimNode]: ...

class VimNode:
    @dataclass
    class StandaloneDocComment:
        text: str

    @dataclass
    class Function:
        name: str
        doc: Optional[str]

class VimPlugin:
    @property
    def content(self) -> List[VimModule]: ...

class VimModule:
    @property
    def path(self) -> str: ...
    @property
    def nodes(self) -> List[VimNode]: ...
