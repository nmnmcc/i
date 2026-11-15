#!/usr/bin/env python3
"""Regression checks for comment boundaries, replay safety and proof strength."""
import importlib.machinery
import importlib.util
import unittest
from pathlib import Path

path = Path(__file__).with_name('doc-comments')
loader = importlib.machinery.SourceFileLoader('doc_comments', str(path))
spec = importlib.util.spec_from_loader(loader.name, loader)
doc = importlib.util.module_from_spec(spec)
loader.exec_module(doc)


class Boundaries(unittest.TestCase):
    def test_rust_strings_are_never_comments(self):
        for literal in ['r#"\n// rule §1\n"#', 'br##"// rule §1"##',
                        'cr#"// rule §1"#', 'c"// rule §1"', '"// rule §1"']:
            source = f'let s = {literal};\n// rule §2\n'
            comments, _ = doc.lex('test.rs', source)
            self.assertEqual(len(comments), 1)
            self.assertNotEqual(doc.strip('test.rs', source),
                                doc.strip('test.rs', source.replace('§1', '')))

    def test_nested_block_comments_and_character_literals(self):
        source = "let c = '/'; let q = '\\\''; /* outer /* nested */ end */ let x = 1;"
        self.assertEqual(len(doc.lex('test.rs', source)[0]), 1)
        self.assertTrue(doc.equivalent('test.rs', source, source.replace('nested', 'reason')))
        self.assertFalse(doc.equivalent('test.rs', source, source.replace("'/'", "'*'")))

    def test_i_interpolation_comments_stay_inside_strings(self):
        for literal in ['"value \\(x // rule §1\n)"', '#"value \\#(x // rule §1\n)"#']:
            source = f's = {literal}\n// rule §2\n'
            comments, _ = doc.lex('test.i', source)
            self.assertEqual(len(comments), 1)
            self.assertFalse(doc.equivalent('test.i', source, source.replace('§1', '')))

    def test_comment_removal_preserves_token_boundaries(self):
        self.assertTrue(doc.equivalent('test.rs', 'let x = 1; // why\n', 'let x = 1;\n'))
        self.assertFalse(doc.equivalent('test.rs', 'a /* reason */ b', 'ab'))
        self.assertFalse(doc.equivalent('test.rs', 'a < < b', 'a << b'))
        self.assertFalse(doc.equivalent('test.rs', 'let s = "a b";', 'let s = "ab";'))

    def test_i_breaks_are_significant(self):
        self.assertTrue(doc.equivalent('test.i', 'x = 1\n// why\ny = 2\n', 'x = 1\ny = 2\n'))
        self.assertFalse(doc.equivalent('test.i', 'f\n(x)', 'f(x)'))
        self.assertFalse(doc.equivalent('test.i', 'x = "a b"', 'x = "ab"'))

    def test_fenced_and_indented_rustdoc_examples_are_exact(self):
        for source in ['/// Why.\n/// ```rust,no_run\n/// let x = 1;\n/// ```\nfn f() {}',
                       '/// Why.\n///\n///     let x = 1;\nfn f() {}']:
            self.assertEqual(len(doc.doc_examples('test.rs', source)), 1)
            self.assertFalse(doc.equivalent('test.rs', source, source.replace('x = 1', 'x = 2')))
            self.assertTrue(doc.equivalent('test.rs', source, source.replace('Why.', 'Constraint.')))

    def test_fixture_directives_are_exact(self):
        source = '// old-syntax-check: E0101\nlet x = 1;\n'
        self.assertFalse(doc.equivalent('test.rs', source, source.replace('E0101', 'E0001')))

    def test_policy_rejects_obsolete_review_labels(self):
        for text in ['literal-types decision 7', 'literal-types decision 12',
                     'type (Decision 1 / EF09)',
                     'the engine of this stage', 'the transition front']:
            self.assertIsNotNone(doc.PROCESS.search(text), text)
        for text in ['trace a compiler stage', 'a decision tree',
                     'the service row must retain its constructor key']:
            self.assertIsNone(doc.PROCESS.search(text), text)

    def test_replay_refuses_stale_or_unsafe_entries(self):
        source = 'let x = 1; // old\n'
        entry = dict(file='test.rs', line=1, hash=doc.hsh(source),
                     old=source, new=source.replace('// old', '// reason'))
        self.assertEqual(doc.prepare([entry], {'test.rs': source})['test.rs'], entry['new'])
        with self.assertRaises(ValueError):
            doc.prepare([entry], {'test.rs': source.replace('1', '2')})
        with self.assertRaises(ValueError):
            doc.prepare([dict(entry, new='let x = 2;\n')], {'test.rs': source})
        with self.assertRaises(ValueError):
            doc.prepare([dict(entry, hash='stale')], {'test.rs': source})
        for path in ['../outside.rs', '/outside.rs', 'Cargo.toml']:
            with self.assertRaises(ValueError):
                doc.prepare([dict(entry, file=path)], {path: source})


if __name__ == '__main__':
    unittest.main()
