from dataclasses import dataclass
from typing import List, Optional

class VimParser:
    def __init__(self): ...
    def parse_plugin_dir(self, path: str) -> VimPlugin: ...
    def parse_module_file(self, path: str) -> VimModule: ...
    def parse_module_str(self, code: str) -> VimModule: ...

class VimNode:
    @dataclass
    class StandaloneDocComment:
        text: str

    @dataclass
    class Function:
        name: str
        args: List[str]
        modifiers: List[str]
        doc: Optional[str]

    @dataclass
    class Command:
        name: str
        modifiers: List[str]
        doc: Optional[str]

    @dataclass
    class Flag:
        name: str
        default_value_token: Optional[str]
        doc: Optional[str]

class VimPlugin:
    @property
    def content(self) -> List[VimModule]: ...

class VimModule:
    @property
    def path(self) -> Optional[str]: ...
    @property
    def doc(self) -> Optional[str]: ...
    @property
    def nodes(self) -> List[VimNode]: ...
