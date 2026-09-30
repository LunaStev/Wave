#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
"""Check binding policy using tokens, independent of Wave source formatting."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
APPROVED_PROVIDER = re.compile(
    r"std/sys/(linux|macos|freebsd)/((amd64|arm64|riscv64)/memory|resolver|interfaces|vector_io|event)\.wave"
    r"|std/sys/wasm/(env|fs|io|memory|process|time)\.wave|std/sys/wasm64/memory\.wave"
)


def tokens(source):
    # Strings/chars remain single tokens; comments (including nested blocks)
    # separate tokens and cannot create declarations. This is not a Wave parser.
    i, line = 0, 1
    while i < len(source):
        ch = source[i]
        if ch.isspace():
            line += ch == '\n'
            i += 1
        elif source.startswith('//', i):
            end = source.find('\n', i)
            i = len(source) if end < 0 else end
        elif source.startswith('/*', i):
            depth, i = 1, i + 2
            while i < len(source) and depth:
                if source.startswith('/*', i):
                    depth, i = depth + 1, i + 2
                elif source.startswith('*/', i):
                    depth, i = depth - 1, i + 2
                else:
                    line += source[i] == '\n'
                    i += 1
            if depth:
                raise ValueError('unterminated block comment')
        elif ch in ('"', "'"):
            quote, start_line, value = ch, line, ''
            i += 1
            while i < len(source) and source[i] != quote:
                if source[i] == '\\':
                    i += 1
                    if i == len(source):
                        break
                    if source[i] == 'x' and re.fullmatch('[0-9a-fA-F]{2}', source[i + 1:i + 3]):
                        value += chr(int(source[i + 1:i + 3], 16))
                        i += 3
                        continue
                    value += {'n': '\n', 'r': '\r', 't': '\t'}.get(source[i], source[i])
                else:
                    value += source[i]
                    line += source[i] == '\n'
                i += 1
            if i == len(source):
                raise ValueError('unterminated literal')
            i += 1
            yield ('string' if quote == '"' else 'char', value, start_line)
        elif ch.isalpha() or ch == '_':
            start = i
            i += 1
            while i < len(source) and (source[i].isalnum() or source[i] == '_'):
                i += 1
            yield ('word', source[start:i], line)
        else:
            yield ('punct', ch, line)
            i += 1


def violations(path, source):
    items = list(tokens(source))
    values = [value for _, value, _ in items]
    std = path.startswith('std/') and not path.startswith('std/libc/')
    for i, (kind, value, line) in enumerate(items):
        if kind != 'word':
            continue
        if std and value == 'extern' and values[i + 1:i + 3] == ['(', 'c']:
            if not APPROVED_PROVIDER.fullmatch(path):
                yield line, 'extern(c) found outside approved C ABI providers'
        if std and value == 'import' and values[i + 1:i + 2] == ['('] and i + 2 < len(items):
            module = items[i + 2]
            if module[0] == 'string' and module[1].startswith('std::libc::'):
                yield line, 'std::libc import found outside std/libc'
        if value == 'let' and (i == 0 or items[i - 1][2] < line or values[i - 1] in ('(', '{', ';', '}')):
            j = i + 1 + (values[i + 1:i + 2] == ['mut'])
            if j + 1 < len(items) and items[j][0] == 'word' and values[j + 1] == ':':
                yield line, 'retired let declaration found'


def main(root=ROOT):
    try:
        found = subprocess.run(['rg', '--files', '--glob', '*.wave', '.'], cwd=root,
                               capture_output=True, text=True, check=False)
        if found.returncode not in (0, 1):
            raise RuntimeError(f'rg search failed (exit {found.returncode}): {found.stderr.strip()}')
        failed = False
        for name in found.stdout.splitlines():
            path = Path(name)
            source = (root / path).read_text(encoding='utf-8')
            for line, message in violations(path.as_posix().removeprefix('./'), source):
                print(f'[FAIL] {path}:{line}: {message}')
                failed = True
        print('[result] FAILED' if failed else '[result] OK')
        return int(failed)
    except (OSError, ValueError, RuntimeError) as error:
        print(f'[FAIL] std policy validation: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
