#include "tree_sitter/parser.h"
#include <assert.h>
#include <stdlib.h>
#include <string.h>
#include <wctype.h>

enum TokenType { BREAK, STRING_START, STRING_CONTENT, STRING_END, ESCAPE, INTERPOLATION_START, INTERPOLATION_END };
typedef struct { uint32_t hashes; uint8_t quotes; bool expression; } String;
typedef struct { String *strings; unsigned length, capacity; } Scanner;

void *tree_sitter_i_external_scanner_create(void) { return calloc(1, sizeof(Scanner)); }
void tree_sitter_i_external_scanner_destroy(void *payload) {
  Scanner *s = payload; free(s->strings); free(s);
}
static void push(Scanner *s, String string) {
  if (s->length == s->capacity) {
    s->capacity = s->capacity ? s->capacity * 2 : 8;
    s->strings = realloc(s->strings, s->capacity * sizeof(String));
    assert(s->strings);
  }
  s->strings[s->length++] = string;
}
unsigned tree_sitter_i_external_scanner_serialize(void *payload, char *buffer) {
  Scanner *s = payload;
  unsigned bytes = s->length * 6;
  assert(bytes <= TREE_SITTER_SERIALIZATION_BUFFER_SIZE);
  for (unsigned i = 0; i < s->length; i++) {
    memcpy(buffer + i * 6, &s->strings[i].hashes, 4);
    buffer[i * 6 + 4] = s->strings[i].quotes;
    buffer[i * 6 + 5] = s->strings[i].expression;
  }
  return bytes;
}
void tree_sitter_i_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  Scanner *s = payload; s->length = 0;
  for (unsigned i = 0; i < length; i += 6) {
    String string;
    memcpy(&string.hashes, buffer + i, 4);
    string.quotes = buffer[i + 4]; string.expression = buffer[i + 5];
    push(s, string);
  }
}
static void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

bool tree_sitter_i_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid) {
  Scanner *s = payload;
  // As in Swift, string scanning pauses for an expression, which the ordinary
  // grammar parses. The delimiter stack also handles strings nested in it.
  if (s->length && !s->strings[s->length - 1].expression) {
    if (!valid[STRING_CONTENT] && !valid[STRING_END] && !valid[ESCAPE] && !valid[INTERPOLATION_START]) return false;
    String string = s->strings[s->length - 1];
    bool content = false;
    lexer->mark_end(lexer);
    while (!lexer->eof(lexer)) {
      if (lexer->lookahead == '"') {
        unsigned quotes = 0;
        while (quotes < string.quotes && lexer->lookahead == '"') { advance(lexer); quotes++; }
        unsigned hashes = 0;
        if (quotes == string.quotes) {
          while (hashes < string.hashes && lexer->lookahead == '#') { advance(lexer); hashes++; }
          if (hashes == string.hashes) {
            if (content) { lexer->result_symbol = STRING_CONTENT; return true; }
            lexer->mark_end(lexer); s->length--; lexer->result_symbol = STRING_END; return true;
          }
        }
      } else if (lexer->lookahead == '\\') {
        advance(lexer);
        unsigned hashes = 0;
        while (lexer->lookahead == '#') { advance(lexer); hashes++; }
        if (hashes == string.hashes && !lexer->eof(lexer)) {
          if (content) { lexer->result_symbol = STRING_CONTENT; return true; }
          if (lexer->lookahead == '(') {
            advance(lexer); lexer->mark_end(lexer);
            s->strings[s->length - 1].expression = true;
            lexer->result_symbol = INTERPOLATION_START; return true;
          }
          if (lexer->lookahead == 'u') {
            advance(lexer);
            if (lexer->lookahead == '{') {
              advance(lexer);
              while (iswxdigit(lexer->lookahead)) advance(lexer);
              if (lexer->lookahead == '}') advance(lexer);
            }
          } else {
            bool cr = lexer->lookahead == '\r'; advance(lexer);
            if (cr && lexer->lookahead == '\n') advance(lexer);
          }
          lexer->mark_end(lexer); lexer->result_symbol = ESCAPE; return true;
        }
      } else { advance(lexer); }
      content = true; lexer->mark_end(lexer);
    }
    lexer->result_symbol = STRING_CONTENT; return content;
  }

  bool newline = false;
  while (iswspace(lexer->lookahead)) {
    newline |= lexer->lookahead == '\n' || lexer->lookahead == '\r';
    lexer->advance(lexer, true);
  }
  lexer->mark_end(lexer);
  int32_t c = lexer->lookahead;
  if (valid[BREAK] && newline) {
    bool begins = iswalnum(c) || c == '_' || c == '"' || c == '#' || c == '(' || c == '[' ||
                  c == '{' || c == '-' || c == '!' || c == '@';
    if (c == '<') { lexer->advance(lexer, true); begins = lexer->lookahead == '<'; }
    else if (c == '.') { lexer->advance(lexer, true); begins = lexer->lookahead == '.'; }
    else if (c == '!') { lexer->advance(lexer, true); begins = lexer->lookahead != '='; }
    else if (c == '-') { lexer->advance(lexer, true); begins = lexer->lookahead != '>'; }
    if (begins) { lexer->result_symbol = BREAK; return true; }
    return false;
  }
  if (valid[INTERPOLATION_END] && c == ')' && s->length && s->strings[s->length - 1].expression) {
    advance(lexer); lexer->mark_end(lexer);
    s->strings[s->length - 1].expression = false;
    lexer->result_symbol = INTERPOLATION_END; return true;
  }
  if (!valid[STRING_START] || (c != '#' && c != '"')) return false;
  String string = {0, 1, false};
  while (lexer->lookahead == '#') { string.hashes++; advance(lexer); }
  if (lexer->lookahead != '"') return false;
  advance(lexer); lexer->mark_end(lexer);
  if (lexer->lookahead == '"') {
    advance(lexer);
    if (lexer->lookahead == '"') {
      advance(lexer);
      // Valid raw triple openers have an immediate line break. Otherwise
      // Swift's same-line raw quote literals retain the single-quote opener.
      if (!string.hashes || lexer->lookahead == '\n' || lexer->lookahead == '\r') {
        string.quotes = 3; lexer->mark_end(lexer);
      }
    }
  }
  push(s, string); lexer->result_symbol = STRING_START; return true;
}
