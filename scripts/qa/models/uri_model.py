# Direct transcription of RFC 3986 5.2.4; this is a model, not std/uri.
import re

def remove_dot_segments(path):
    out = ''
    while path:
        if path.startswith('../'):
            path = path[3:]
        elif path.startswith('./'):
            path = path[2:]
        elif path.startswith('/./'):
            path = path[2:]
        elif path == '/.':
            path = '/'
        elif path.startswith('/../') or path == '/..':
            path = '/' if path == '/..' else path[3:]
            out = out.rsplit('/', 1)[0] if '/' in out else ''
        elif path in ('.', '..'):
            path = ''
        else:
            end = path.find('/', 1 if path.startswith('/') else 0)
            if end < 0:
                end = len(path)
            out += path[:end]
            path = path[end:]
    return out

assert remove_dot_segments('/a/b/c/./../../g') == '/a/g'
assert remove_dot_segments('mid/content=5/../6') == 'mid/6'
for path in ('/.//h', '/a/..//h', '/.//a:b'):
    result = remove_dot_segments(path)
    print(f'base=x:a reference={path!r}: scheme=x authority=None path={result!r} rendered={"x:"+result!r}')
    assert result.startswith('//')
