"""Fence reader shared by the documentation parser check and example runner."""
from dataclasses import dataclass
from pathlib import PurePosixPath
import re
import shlex


@dataclass(frozen=True)
class Fence:
    info: str
    source: str
    line: int
    closed: bool


def fences(text):
    """Read Markdown fences, including longer backtick and tilde fences.

    A fence inside another fence is content, not a second example. Remove
    up to the opening fence's indentation, as Markdown does.
    """
    lines = text.splitlines(keepends=True)
    index = 0
    while index < len(lines):
        opening = re.fullmatch(r'( {0,3})(`{3,}|~{3,})([^\n]*)\n?', lines[index])
        if not opening or (opening[2][0] == '`' and '`' in opening[3]):
            index += 1
            continue
        start = index
        index += 1
        source = []
        closing = re.compile(r' {0,3}' + re.escape(opening[2][0])
                             + '{' + str(len(opening[2])) + r',}[ \t]*\n?')
        while index < len(lines) and not closing.fullmatch(lines[index]):
            line = lines[index]
            spaces = len(line) - len(line.lstrip(' '))
            source.append(line[min(spaces, len(opening[1])):])
            index += 1
        yield Fence(opening[3].strip(), ''.join(source), start + 1, index < len(lines))
        index += 1


def i_info(info):
    """Return (kind, file path, codes), or None for a different language.

    title= is a file= alias when used alone. A matching title= on a file=
    block also lets Fumadocs display its filename without a site change.
    """
    if not re.match(r'^i(?:\s|$)', info):
        return None
    words = shlex.split(info)
    words.pop(0)
    titles = [word[6:] for word in words if word.startswith('title=')]
    words = [word for word in words if not word.startswith('title=')]
    if len(titles) > 1 or (titles and not titles[0]):
        raise ValueError('expected at most one nonempty title=')
    if not words:
        return ('file', titles[0], frozenset()) if titles else ('fragment', None, frozenset())
    if words[0].startswith('file=') and len(words) == 1:
        path = words[0][5:]
        if titles and titles[0] != path:
            raise ValueError('file= and title= must name the same file')
        if not path:
            raise ValueError('file= needs a relative path')
        return 'file', path, frozenset()
    if words[0] in ('run', 'test', 'check') and len(words) == 1:
        return words[0], None, frozenset()
    if words[0] == 'error' and len(words) > 1 and all(
            re.fullmatch(r'[EW]\d{4}', word) for word in words[1:]):
        return 'error', None, frozenset(words[1:])
    raise ValueError('expected run, test, check, error <codes…>, or file=<path>; got '
                     + repr(' '.join(words)))


def safe_file(path):
    parts = PurePosixPath(path)
    if (not path or parts.is_absolute() or '..' in parts.parts or '\\' in path
            or parts.as_posix() != path or path == '.'):
        raise ValueError('expected a project-relative file path without . or ..; got ' + repr(path))
    return path


def generated(path):
    return 'generated' in path.parts
