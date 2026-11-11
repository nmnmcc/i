#include "tree_sitter/parser.h"
#include <wctype.h>

enum TokenType { BREAK, STRING_CONTENT };

// A break is possible only in parser states after an expression-ending token.
// Look across whitespace/comments for the statement-starting token on its right.
void *tree_sitter_i_external_scanner_create(void) { return NULL; }
void tree_sitter_i_external_scanner_destroy(void *payload) { (void)payload; }
unsigned tree_sitter_i_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload; (void)buffer; return 0;
}
void tree_sitter_i_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  (void)payload; (void)buffer; (void)length;
}
bool tree_sitter_i_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid) {
  (void)payload;
  // tree-sitter-gleam's quoted-content approach prevents extras (especially
  // // comments) from competing with the interior of a string.
  if (valid[STRING_CONTENT]) {
    bool content = false;
    while (lexer->lookahead != '"' && lexer->lookahead != '\\') {
      if (lexer->eof(lexer)) return false;
      content = true;
      lexer->advance(lexer, false);
    }
    lexer->result_symbol = STRING_CONTENT;
    return content;
  }
  if (!valid[BREAK]) return false;
  bool newline = false;
  while (iswspace(lexer->lookahead)) {
    newline |= lexer->lookahead == '\n' || lexer->lookahead == '\r';
    lexer->advance(lexer, true);
  }
  // Comments are extras. Their terminating newline is scanned after the comment.
  lexer->mark_end(lexer);
  int32_t c = lexer->lookahead;
  bool begins = iswalnum(c) || c == '_' || c == '"' || c == '(' || c == '[' ||
                c == '{' || c == '-' || c == '!' || c == '@';
  if (c == '<') {
    lexer->advance(lexer, true); begins = lexer->lookahead == '<';
  } else if (c == '.') {
    lexer->advance(lexer, true); begins = lexer->lookahead == '.';
  } else if (c == '!') {
    lexer->advance(lexer, true); begins = lexer->lookahead != '=';
  } else if (c == '-') {
    lexer->advance(lexer, true); begins = lexer->lookahead != '>';
  }
  if (!newline || !begins) return false;
  lexer->result_symbol = BREAK;
  return true;
}
