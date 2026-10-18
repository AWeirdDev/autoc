/*
    autoc.h - Interface for C++/CX autocompletion.

    Note: this is **DEPRECATED**; It's here mainly for reference reasons.
          Don't blame me for not using git for version control.
 */

#ifndef AUTOC_H
#define AUTOC_H

#include <clang-c/Index.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * A parsed source document and its associated libclang resources.
 *
 * The caller owns the document buffer and path. They must remain valid
 * for as long as the document uses them.
 *
 * If parsing fails, translation_unit is NULL.
 */
typedef struct {
    CXTranslationUnit translation_unit;
    CXIndex index;
    const char *path;
    unsigned long len;
    const char *buf;
} autoc_Doc;

/*
 * Creates a document and parses the specified source file.
 *
 * file: Path to the source file.
 * len:  Length of buf in bytes.
 * buf:  Current source contents, used for subsequent reparsing and
 *       code completion.
 * argc: Number of compiler arguments.
 * args: Compiler argument array.
 *
 * Returns a document whose translation_unit may be NULL if parsing fails.
 */
autoc_Doc autoc_Doc_new(const char *file, unsigned long len, const char *buf,
                        int argc, const char *const *args);

/*
 * Result of reparsing a document.
 *
 * Zero indicates success; nonzero values are CXErrorCode values.
 */
typedef enum CXErrorCode autoc_Doc_ReparseResult;

/*
 * Reparses the document using its current buffer and length.
 *
 * The caller must update buf and len before calling this function if
 * the source contents have changed.
 */
autoc_Doc_ReparseResult autoc_Doc_reparse(autoc_Doc *doc);

/*
 * Releases the document's translation unit and index.
 *
 * The document must not be used afterward.
 */
void autoc_Doc_dispose(autoc_Doc doc);

typedef enum {
    autoc_Completion_Chunk_End,
    autoc_Completion_Chunk_TypedText,
    autoc_Completion_Chunk_Text,
    autoc_Completion_Chunk_ResultType,
    autoc_Completion_Chunk_Placeholder,
    autoc_Completion_Chunk_CurrentParam,
    autoc_Completion_Chunk_Informative,
    autoc_Completion_Chunk_Punct,
    autoc_Completion_Chunk_Optional
} autoc_Completion_Chunk_Kind;

typedef struct autoc_Completion_Chunk autoc_Completion_Chunk;

typedef struct {
    autoc_Completion_Chunk *items;
    size_t count;
    size_t cap;
} autoc_Completion_Chunks;

/*
 * A completion chunk.
 *
 * For autoc_Completion_Chunk_Optional, use as.optional.
 * For every other kind, use as.text.
 */
struct autoc_Completion_Chunk {
    autoc_Completion_Chunk_Kind kind;

    union {
        char *text;
        autoc_Completion_Chunks optional;
    } as;
};

typedef struct {
    autoc_Completion_Chunks chunks;
    unsigned priority;
    enum CXAvailabilityKind availability;
    enum CXCursorKind cursor_kind;
    char *brief;  // may be NULL
} autoc_Completion;

typedef struct {
    CXCodeCompleteResults *inner;
    unsigned cursor;
} autoc_Completions;

/*
 * Requests code-completion results at the specified 1-based line and
 * column in the document.
 *
 * Results are sorted by libclang's completion ordering.
 *
 * If code completion fails, the returned inner pointer is NULL.
 */
autoc_Completions autoc_Doc_code_complete(autoc_Doc *doc, unsigned line,
                                          unsigned column);

/*
 * Retrieves the next available completion.
 *
 * Returns:
 *   0  A completion was written to *out.
 *  -1  No more available completions remain.
 *
 * The caller must release a successfully retrieved completion with
 * autoc_Completion_free().
 */
int autoc_Completions_next(autoc_Completions *completions,
                           autoc_Completion *out);

/*
 * Frees the chunks and brief comment owned by a completion.
 */
void autoc_Completion_free(autoc_Completion completion);

/*
 * Releases the underlying libclang completion results.
 *
 * The completions object must not be used afterward.
 */
void autoc_Completions_dispose(autoc_Completions completions);

#ifdef __cplusplus
}
#endif

#endif /* AUTOC_H */
