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


def i_options(info):
    """Separate execution words from the filename and package attributes."""
    words, options = [], {}
    for word in shlex.split(info)[1:]:
        key, separator, value = word.partition('=')
        if separator and key in ('title', 'package', 'file'):
            if key in options or not value:
                raise ValueError(f'expected one nonempty {key}=')
            options[key] = value
        else:
            words.append(word)
    package = options.get('package')
    if package is not None and (not re.fullmatch(r'_?[a-z][a-z0-9_]*', package) or package == 'i'):
        raise ValueError('package= must be a package name other than i; got ' + repr(package))
    return words, options


def i_info(info):
    """Return (kind, file path, codes), or None for a different language.

    A title names the primary source of an executable block. On a supporting
    file block it is also a file= alias. file= remains required on shown.
    """
    if not re.match(r'^i(?:\s|$)', info):
        return None
    words, options = i_options(info)
    path = options.get('file')
    title = options.get('title')
    if path and title and title != path:
        raise ValueError('file= and title= must name the same file')
    if not words:
        if 'package' in options:
            raise ValueError('package= needs an example kind')
        return ('file', path or title, frozenset()) if path or title else ('fragment', None, frozenset())
    if words[0] in ('shown', 'shown-unchecked') and len(words) == 1:
        if words[0] == 'shown' and not path:
            raise ValueError('shown needs file=<path>')
        return words[0], path or title or 'src/main.i', frozenset()
    if path:
        raise ValueError('file= belongs to a supporting file or a shown block')
    if words[0] in ('run', 'test', 'test-fails', 'check') and len(words) == 1:
        return words[0], title, frozenset()
    if words[0] == 'error' and len(words) > 1 and all(
            re.fullmatch(r'[EW]\d{4}', word) for word in words[1:]):
        return 'error', title, frozenset(words[1:])
    raise ValueError('expected run, test, test-fails, check, error <codes…>, shown file=<path>, shown-unchecked, or file=<path>; got '
                     + repr(' '.join(words)))


def safe_file(path):
    parts = PurePosixPath(path)
    if (not path or parts.is_absolute() or '..' in parts.parts or '\\' in path
            or parts.as_posix() != path or path == '.'):
        raise ValueError('expected a project-relative file path without . or ..; got ' + repr(path))
    return path


def generated(path):
    return 'generated' in path.parts
