"""Keep comment-transfer APIs on the native tree; preserve old oracles privately."""
from pathlib import Path
import json, subprocess
ROOT=Path.cwd()

def apply():
    proof=json.loads((ROOT/'target/fa-night/step3-green.json').read_text())
    assert proof['commit']==subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()
    base=ROOT/'crates/ilang-syntax/src'
    # The old-row dispatcher is inactive once ROWS3 is on; the one-tree
    # build keeps only the native reader and its shared document entry point.
    import importlib.util
    helper_path=ROOT/'scripts/syn3-delete-resolver.py'
    spec=importlib.util.spec_from_file_location('resolver_delete',helper_path)
    helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
    path=base/'lisp/syntax.rs'; text=path.read_text()
    if 'fn show_as_old_rows(' in text:
        text=helper.remove_function(text,'show_as_old_rows','    ')
    text=text.replace('''        if ROWS3 {
            self.show_as_rows3(position)
        } else {
            self.show_as_old_rows(position)
        }''', '        self.show_as_rows3(position)')
    path.write_text(text)
    path=base/'rows.rs';text=path.read_text()
    if 'pub fn from_document(' not in text:
        start=text.index('pub fn from_lisp(source:')
        at=text.index('    let mut module = Module::default();',start)
        text=text[:at]+'''    from_document(&document)
}

/// Read a document already parsed with the final Lisp rows.
pub fn from_document(document: &crate::lisp::Document) -> (Module, Vec<Diagnostic>) {
'''+text[at:]
    path.write_text(text)
    path=base/'surface.rs'
    text=path.read_text()+'''
/// Transfer comments for an already-read native module. The source spans of
/// that module and the spans of its printed/read-back tree identify anchors.
pub(crate) fn module_comments(
    source: &str, module: &Module, output: String, input_lisp: bool, output_lisp: bool,
) -> (String, Vec<Span>) {
    let input = if input_lisp { lex3_lisp(source) } else { lex3(source) };
    let (back, _) = if output_lisp { crate::rows::from_lisp(&output) }
        else { crate::parse::parse_module(&output) };
    let printed = if output_lisp { lex3_lisp(&output) } else { lex3(&output) };
    let from = anchors(module, &input);
    let to = anchors(&back, &printed);
    transfer(source, &input, output, &from, &to)
}
'''
    path.write_text(text)
    path=base/'lisp/mod.rs'
    text=path.read_text().replace('mod comments;\n','')
    text=text.replace('pub use from_ast::{print_module, print_with_comments};\n','')
    text+='''
/// Print final syntax rows from the compiler's one tree.
pub fn print_module(module: &crate::ast::Module) -> String {
    crate::deep::deep(|| crate::rows::to_lisp(module))
}
pub fn print_with_comments(source: &str, module: &crate::ast::Module) -> (String, Vec<Span>) {
    crate::deep::deep(|| crate::surface::module_comments(
        source, module, crate::rows::to_lisp(module), false, true,
    ))
}
'''
    path.write_text(text)
    (base/'lisp/comments.rs').unlink(missing_ok=True)
    path=base/'lisp/to_source.rs'
    text=path.read_text()+'''
/// Convert final rows and transfer comments at their original source anchors.
pub fn normal_with_comments(source: &str, document: &Document) -> Result<(String, Vec<Span>)> {
    crate::deep::deep(|| {
        let (module, errors) = crate::rows::from_document(document);
        if let Some(error) = errors.into_iter().next() { return Err(error); }
        let output = normal_source3(document)?;
        Ok(crate::surface::module_comments(source, &module, output, true, false))
    })
}
'''
    # Keep the ten-thousand-level historical renderer oracle unchanged.
    # The flat many-comment case exercises the production API after changing
    # only the empty parameter group to the final §6.8 data-pattern row.
    for name in ['many_comment_insertions_keep_source_order']:
        begin=text.index('    fn '+name+'(')
        end=text.find('    #[test]',begin)
        if end < 0: end=text.index('\n}',begin)
        body=text[begin:end]
        body=body.replace('normal_with_comments(', 'super::normal_with_comments(')
        body=body.replace('(= f (-> () _ {', '(= f (-> (,) _ {')
        text=text[:begin]+body+text[end:]
    text += r'''
#[cfg(test)]
mod native_comment_tests {
    #[test]
    fn native_comments_keep_nested_statement_anchors() {
        let source = "(= value (f {\n// outer\n(= v (g {\n// inner\n(= w x) w})) v}))";
        let document = crate::lisp::parse(source).unwrap();
        let (output, lost) = super::normal_with_comments(source, &document).unwrap();
        assert!(lost.is_empty(), "{lost:?}");
        assert_eq!(output.matches("// outer").count(), 1);
        assert_eq!(output.matches("// inner").count(), 1);
        assert!(output.contains("// outer\nv = "), "{output}");
        assert!(output.contains("// inner\nw = "), "{output}");
        let (_, errors) = crate::parse_module(&output);
        assert!(errors.is_empty(), "{errors:?}");
    }
}
'''
    path.write_text(text)

if __name__=='__main__':
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--apply',action='store_true')
    if p.parse_args().apply:apply()
    else:print('Preparation only; require the exact green step3 proof before applying.')
