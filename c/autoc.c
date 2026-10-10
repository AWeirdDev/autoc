/*
        autoc - Interface for C++/CX autocompletion.
 */

#include <assert.h>
#include <clang-c/Index.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "clang-c/CXErrorCode.h"

typedef struct {
    CXTranslationUnit translation_unit;
    CXIndex index;
    const char *path;
    unsigned long len;
    const char *buf;
} autoc_Doc;

/**
 * Creates a new `Doc`.
 *
 * If the parsing fails, upon inspecting `translation_unit`,
 * you'll see that it's a `NULL` (0).
 */
autoc_Doc autoc_Doc_new(const char *file, unsigned long len, const char *buf,
                        int argc, const char *const *args) {
    CXIndex idx = clang_createIndex(/* excludeDeclarationsFromPCH */ 0,
                                    /* displayDiagnostics */ 0);

    unsigned parse_opts =
        CXTranslationUnit_PrecompiledPreamble |
        CXTranslationUnit_CacheCompletionResults |
        CXTranslationUnit_IncludeBriefCommentsInCodeCompletion;

    CXTranslationUnit tu =
        clang_parseTranslationUnit(idx, file, args, argc, NULL, 0, parse_opts);

    return (autoc_Doc){.translation_unit = tu,
                       .index = idx,
                       .path = file,
                       .len = len,
                       .buf = buf};
}

typedef enum CXErrorCode autoc_Doc_ReparseResult;

/**
 * Reparses the document, usually after editing.
 *
 * If this operation fails, a non-zero error code will be returned,
 * which is described by the enum `CXErrorCode`.
 */
autoc_Doc_ReparseResult autoc_Doc_reparse(autoc_Doc *doc) {
    struct CXUnsavedFile unsaved = {
        .Filename = doc->path, .Contents = doc->buf, .Length = doc->len};
    return clang_reparseTranslationUnit(
        doc->translation_unit, 1, &unsaved,
        clang_defaultReparseOptions(doc->translation_unit));
}

/**
 * Disposes the given `Doc`'s translation unit and the index it's in.
 */
void autoc_Doc_dispose(autoc_Doc doc) {
    clang_disposeTranslationUnit(doc.translation_unit);
    clang_disposeIndex(doc.index);
}

typedef struct {
    CXCodeCompleteResults *inner;
    unsigned cursor;
} autoc_Completions;

autoc_Completions autoc_Doc_code_complete(autoc_Doc *doc, unsigned line,
                                          unsigned column) {
    struct CXUnsavedFile unsaved_file = {
        .Filename = doc->path, .Contents = doc->buf, .Length = doc->len};
    CXCodeCompleteResults *res = clang_codeCompleteAt(
        doc->translation_unit, doc->path, line, column, &unsaved_file, 1,
        clang_defaultCodeCompleteOptions());
    if (!res) {
        return (autoc_Completions){NULL, 0};
    }

    clang_sortCodeCompletionResults(res->Results, res->NumResults);
    return (autoc_Completions){.inner = res, .cursor = 0};
}

typedef enum {
    // end of chunk
    autoc_Completion_Chunk_End,

    // the part the user is typing or what gets inserted
    autoc_Completion_Chunk_TypedText,

    autoc_Completion_Chunk_Text,
    autoc_Completion_Chunk_ResultType,

    // a parameter to fill in
    autoc_Completion_Chunk_Placeholder,

    autoc_Completion_Chunk_CurrentParam,

    // display-only, wont be inserted
    autoc_Completion_Chunk_Informative,

    // punctuation
    autoc_Completion_Chunk_Punct,

    // nested
    autoc_Completion_Chunk_Optional,
} autoc_Completion_Chunk_Kind;

typedef struct autoc_Completion_Chunk autoc_Completion_Chunk;

typedef struct {
    autoc_Completion_Chunk *items;
    size_t count, cap;
} autoc_Completion_Chunks;

struct autoc_Completion_Chunk {
    autoc_Completion_Chunk_Kind kind;
    union {
        char *text;
        autoc_Completion_Chunks optional;
    } as;
};

static autoc_Completion_Chunk_Kind autoc__kind_from_clang(
    enum CXCompletionChunkKind k) {
    switch (k) {
        case CXCompletionChunk_TypedText:
            return autoc_Completion_Chunk_TypedText;
        case CXCompletionChunk_Text:
            return autoc_Completion_Chunk_Text;
        case CXCompletionChunk_ResultType:
            return autoc_Completion_Chunk_ResultType;
        case CXCompletionChunk_Placeholder:
            return autoc_Completion_Chunk_Placeholder;
        case CXCompletionChunk_CurrentParameter:
            return autoc_Completion_Chunk_CurrentParam;
        case CXCompletionChunk_Informative:
            return autoc_Completion_Chunk_Informative;
        case CXCompletionChunk_Optional:
            return autoc_Completion_Chunk_Optional;
        default:
            return autoc_Completion_Chunk_Punct;
    }
}

typedef struct {
    autoc_Completion_Chunks chunks;
    unsigned priority;
    enum CXAvailabilityKind availability;
    enum CXCursorKind cursor_kind;
    char *brief;  // may be NULL
} autoc_Completion;

static void autoc__chunks_push(autoc_Completion_Chunks *cs,
                               autoc_Completion_Chunk c) {
    if (cs->count == cs->cap) {
        cs->cap = cs->cap ? cs->cap * 2 : 8;
        cs->items = realloc(cs->items, cs->cap * sizeof *cs->items);
        assert(cs->items);
    }
    cs->items[cs->count++] = c;
}

static autoc_Completion_Chunks autoc__chunks_from_clang(
    CXCompletionString completion_str) {
    unsigned n_completion_chunks = clang_getNumCompletionChunks(completion_str);
    autoc_Completion_Chunks out = {0};

    for (unsigned chunk_index = 0; chunk_index < n_completion_chunks;
         chunk_index++) {
        enum CXCompletionChunkKind chunk_kind =
            clang_getCompletionChunkKind(completion_str, chunk_index);

        autoc_Completion_Chunk compl_chunk = {0};
        compl_chunk.kind = autoc__kind_from_clang(chunk_kind);
        if (compl_chunk.kind == autoc_Completion_Chunk_Optional) {
            compl_chunk.as.optional = autoc__chunks_from_clang(
                clang_getCompletionChunkCompletionString(completion_str,
                                                         chunk_index));
        } else {
            CXString t =
                clang_getCompletionChunkText(completion_str, chunk_index);
            const char *txt = clang_getCString(t);
            compl_chunk.as.text = strdup(txt ? txt : "");
            clang_disposeString(t);
        }
        autoc__chunks_push(&out, compl_chunk);
    }

    return out;
}

int autoc_Completions_next(autoc_Completions *completions,
                           autoc_Completion *out) {
    while (completions->inner &&
           completions->cursor < completions->inner->NumResults) {
        CXCompletionResult *result =
            &completions->inner->Results[completions->cursor++];
        CXCompletionString s = result->CompletionString;

        if (clang_getCompletionAvailability(s) != CXAvailability_Available) {
            continue;
        }

        *out = (autoc_Completion){
            .chunks = autoc__chunks_from_clang(s),
            .priority = clang_getCompletionPriority(s),
            .availability = CXAvailability_Available,
            .cursor_kind = result->CursorKind,
        };

        CXString b = clang_getCompletionBriefComment(s);
        const char *bs = clang_getCString(b);
        out->brief = (bs && *bs) ? strdup(bs) : NULL;
        clang_disposeString(b);
        return 0;
    }

    return -1;
}

static void autoc__chunks_free(autoc_Completion_Chunks *cs) {
    for (size_t i = 0; i < cs->count; i++) {
        autoc_Completion_Chunk *c = &cs->items[i];
        if (c->kind == autoc_Completion_Chunk_Optional)
            autoc__chunks_free(&c->as.optional);
        else
            free(c->as.text);
    }
    free(cs->items);
    *cs = (autoc_Completion_Chunks){0};
}

void autoc_Completion_free(autoc_Completion c) {
    autoc__chunks_free(&c.chunks);
    free(c.brief);
}

void autoc_Completions_dispose(autoc_Completions completions) {
    clang_disposeCodeCompleteResults(completions.inner);
}

// void emscripten_longjmp(uintptr_t env, int val) {
//     (void)env;
//     (void)val;
//     abort();
// }
