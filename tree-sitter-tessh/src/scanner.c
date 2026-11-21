// tessh's "…" strings: their $(…) and ${…} may hold brackets and strings
// of their own (`"${len "x"}"`), which a regular token can't follow.
#include "tree_sitter/parser.h"
#include <stdbool.h>

enum { DQ_STRING };

static void next(TSLexer *l) { l->advance(l, false); }

static bool dq_body(TSLexer *l, int depth);

// Up to the `close` of a group whose opening was just read.
static bool group(TSLexer *l, int32_t close, int depth) {
    if (depth > 64) return false;
    while (!l->eof(l)) {
        int32_t c = l->lookahead;
        if (c == close) { next(l); return true; }
        if (c == '\\') { next(l); if (!l->eof(l)) next(l); continue; }
        if (c == '"') { next(l); if (!dq_body(l, depth + 1)) return false; continue; }
        if (c == '\'') {
            next(l);
            while (!l->eof(l) && l->lookahead != '\'') next(l);
            if (l->eof(l)) return false;
            next(l);
            continue;
        }
        next(l);
        if (c == '(' && !group(l, ')', depth + 1)) return false;
        if (c == '[' && !group(l, ']', depth + 1)) return false;
        if (c == '{' && !group(l, '}', depth + 1)) return false;
    }
    return false;
}

// A string's rest, its opening `"` read: `""` empty, `"""` long.
static bool dq_body(TSLexer *l, int depth) {
    bool triple = false;
    if (l->lookahead == '"') {
        next(l);
        if (l->lookahead != '"') return true;
        next(l);
        triple = true;
    }
    while (!l->eof(l)) {
        int32_t c = l->lookahead;
        if (c == '"') {
            next(l);
            if (!triple) return true;
            if (l->lookahead == '"') {
                next(l);
                if (l->lookahead == '"') { next(l); return true; }
            }
            continue;
        }
        next(l);
        if (c == '\\') { if (!l->eof(l)) next(l); continue; }
        if (c == '$' && (l->lookahead == '(' || l->lookahead == '{')) {
            int32_t close = l->lookahead == '(' ? ')' : '}';
            next(l);
            if (!group(l, close, depth + 1)) return false;
        }
    }
    return false;
}

void *tree_sitter_tessh_external_scanner_create(void) { return NULL; }
void tree_sitter_tessh_external_scanner_destroy(void *p) { (void)p; }
unsigned tree_sitter_tessh_external_scanner_serialize(void *p, char *b) { (void)p; (void)b; return 0; }
void tree_sitter_tessh_external_scanner_deserialize(void *p, const char *b, unsigned n) { (void)p; (void)b; (void)n; }

bool tree_sitter_tessh_external_scanner_scan(void *p, TSLexer *l, const bool *valid) {
    (void)p;
    if (!valid[DQ_STRING]) return false;
    while (l->lookahead == ' ' || l->lookahead == '\t') l->advance(l, true);
    if (l->lookahead != '"') return false;
    next(l);
    if (!dq_body(l, 0)) return false;
    l->result_symbol = DQ_STRING;
    return true;
}
