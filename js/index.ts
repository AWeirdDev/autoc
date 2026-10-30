// abi --

namespace abi {
    declare const __brand: unique symbol;
    export type DocId = number & {
        [__brand]: typeof __brand;
    };

    export declare const __CStr: unique symbol;

    /**
     * A pointer.
     *
     * This has no runtime meaning, and is solely here to make things
     * clearer when crossing the WebAssembly boundary.
     */
    export type UnsafePtr<Type> = number & {
        [__brand]: Type;
    };

    export interface EmscriptenFS {
        writeFile(path: string, data: string | ArrayBufferView): void;
        mkdirTree(path: string): void;
        unlink(path: string): void;
    }

    export interface EmscriptenModule {
        // emscripten rt
        FS: EmscriptenFS;
        UTF8ToString(
            ptr: UnsafePtr<typeof __CStr>,
            maxBytesToRead?: number,
        ): string;
        stringToNewUTF8(str: string): UnsafePtr<typeof __CStr>;

        // malloc
        _free(ptr: UnsafePtr<any>): void;

        _doc_new(
            pathPtr: UnsafePtr<typeof __CStr>,
            srcPtr: UnsafePtr<typeof __CStr>,
            flagsPtr: UnsafePtr<typeof __CStr>,
        ): DocId | DocNewError;
        _doc_update(
            docId: DocId,
            srcPtr: UnsafePtr<typeof __CStr>,
        ): 0 | DocUpdateError;
        _doc_code_complete(
            docId: DocId,
            line: number,
            col: number,
        ): UnsafePtr<typeof __CStr>;

        /**
         * Disposes the document.
         * This function is guaranteed not to fail.
         */
        _doc_dispose(docId: DocId): void;
    }

    enum CXError {
        /**
         * A generic error code, no further details are available.
         */
        Failure = -2,

        /**
         * libclang crashed while performing the requested operation.
         */

        Crashed = -3,

        /**
         * The function detected that the arguments violate the function contract.
         */
        InvalidArguments = -4,

        /**
         * An AST deserialization error has occurred.
         */
        ASTReadError = -5,
    }

    enum DocNewError_With {
        /**
         * Found unexpected null byte
         */
        NulError = -1,
    }
    export const DocNewError = {
        ...CXError,
        ...DocNewError_With,
    } as const;
    export type DocNewError = (typeof DocNewError)[keyof typeof DocNewError];

    enum DocUpdateError_With {
        /**
         * The requested document is not found.
         */
        NotFound,
    }
    export const DocUpdateError = {
        ...CXError,
        ...DocUpdateError_With,
    } as const;
    export type DocUpdateError =
        (typeof DocUpdateError)[keyof typeof DocUpdateError];
}

// -- abi

// public api --

/**
 * The autoc engine.
 */
export class AutoC {
    /**
     * The WebAssembly module.
     */
    module!: abi.EmscriptenModule;

    // use createAutoC()
    private constructor() {}

    /**
     * Create a new document.
     */
    createDoc({ path, source, flags }: CreateDocOptions): Doc {
        const rawPath = this.module.stringToNewUTF8(path);
        const rawSource = this.module.stringToNewUTF8(source || "");
        const rawFlags = this.module.stringToNewUTF8(
            flags ? flags.join("\n") : "",
        );

        const docId = this.module._doc_new(rawPath, rawSource, rawFlags);

        this.module._free(rawPath);
        this.module._free(rawFlags);

        // let it leak let it leak
        // this.module._free(rawSource);

        if ((docId as number) < 0) {
            // TODO: prettier errors?
            throw new Error(abi.DocNewError[docId as number]);
        }
        return new Doc(this.module, docId as abi.DocId, rawSource);
    }

    /**
     * Add a file to the FS for libclang to read from.
     */
    addFile(path: string, data: string | ArrayBufferView<ArrayBufferLike>) {
        this.module.FS.writeFile(path, data);
    }

    /**
     * Make a directory.
     */
    makeDir(path: string) {
        this.module.FS.mkdirTree(path);
    }
}

interface CreateDocOptions {
    /**
     * The path of the source file. It will be the file name
     * displayed on diagnostics and more.
     *
     * Typically, `main.c` or `main.cpp` is enough.
     */
    path: string;

    /**
     * The initial source text. (optional)
     */
    source?: string;

    /**
     * Compilation flags. (optional)
     *
     * For example, C++20 with a `wasm32-wasi` target:
     *
     * ```js
     * ["-x", "c++", "-std=c++20", "--target=wasm32-wasi"]
     * ```
     */
    flags?: string[];
}

/**
 * Initializes autoc. It should be reused throughout
 * the session.
 */
export async function createAutoC(): Promise<AutoC> {
    const module = (await // @ts-ignore
    (await import("./wasm")).default()) as abi.EmscriptenModule;

    return Object.setPrototypeOf({ module }, AutoC.prototype);
}

export class Doc {
    #module: abi.EmscriptenModule;
    #id: abi.DocId;
    #buf: abi.UnsafePtr<typeof abi.__CStr>;

    constructor(
        module: abi.EmscriptenModule,
        id: abi.DocId,
        srcBuf: abi.UnsafePtr<typeof abi.__CStr>,
    ) {
        this.#module = module;
        this.#id = id;
        this.#buf = srcBuf;
    }

    update(newSource: string) {
        this.#module._free(this.#buf);
        this.#buf = this.#module.stringToNewUTF8(newSource);
        this.#module._doc_update(this.#id, this.#buf);
    }

    /**
     * Perform code completion, giving an array of {@link CodeCompletion}.
     */
    codeComplete(line: number, column: number): CodeCompletion[] {
        const resultStringPtr = this.#module._doc_code_complete(
            this.#id,
            line,
            column,
        );
        const resultStringView = this.#module.UTF8ToString(resultStringPtr);
        this.#module._free(resultStringPtr);

        return JSON.parse(resultStringView);
    }

    [Symbol.dispose]() {
        this.#module._free(this.#buf);
        this.#module._doc_dispose(this.#id);
    }
}

export type CodeCompletionChunks = CodeCompletionChunk[];

export type CodeCompletionChunk =
    | { type: "optional"; data: CodeCompletionChunks }
    | { type: "typedText"; data: string }
    | { type: "text"; data: string }
    | { type: "placeholder"; data: string }
    | { type: "informative"; data: string }
    | { type: "currentParameter"; data: string }
    | { type: "leftParen"; data: string }
    | { type: "rightParen"; data: string }
    | { type: "leftBracket"; data: string }
    | { type: "rightBracket"; data: string }
    | { type: "leftBrace"; data: string }
    | { type: "rightBrace"; data: string }
    | { type: "leftAngle"; data: string }
    | { type: "rightAngle"; data: string }
    | { type: "comma"; data: string }
    | { type: "resultType"; data: string }
    | { type: "colon"; data: string }
    | { type: "semiColon"; data: string }
    | { type: "equal"; data: string }
    | { type: "horizontalSpace"; data: string }
    | { type: "verticalSpace"; data: string };

export interface CodeCompletion {
    /**
     * Completion chunks.
     */
    chunks: CodeCompletionChunks;

    /**
     * Determine the priority of this code completion.
     *
     * The lower the value, the higher the priority.
     */
    priority: number;

    /**
     * Kind of entity that the cursor refers to.
     */
    cursorKind: CXCursorKind;

    /**
     * Brief documentation comment attached to the declaration.
     */
    brief: string;
}

// @generated
export enum CXCursorKind {
    UnexposedDecl = 1,
    StructDecl = 2,
    UnionDecl = 3,
    ClassDecl = 4,
    EnumDecl = 5,
    FieldDecl = 6,
    EnumConstantDecl = 7,
    FunctionDecl = 8,
    VarDecl = 9,
    ParmDecl = 10,
    ObjCInterfaceDecl = 11,
    ObjCCategoryDecl = 12,
    ObjCProtocolDecl = 13,
    ObjCPropertyDecl = 14,
    ObjCIvarDecl = 15,
    ObjCInstanceMethodDecl = 16,
    ObjCClassMethodDecl = 17,
    ObjCImplementationDecl = 18,
    ObjCCategoryImplDecl = 19,
    TypedefDecl = 20,
    CxxMethod = 21,
    Namespace = 22,
    LinkageSpec = 23,
    Constructor = 24,
    Destructor = 25,
    ConversionFunction = 26,
    TemplateTypeParameter = 27,
    NonTypeTemplateParameter = 28,
    TemplateTemplateParameter = 29,
    FunctionTemplate = 30,
    ClassTemplate = 31,
    ClassTemplatePartialSpecialization = 32,
    NamespaceAlias = 33,
    UsingDirective = 34,
    UsingDeclaration = 35,
    TypeAliasDecl = 36,
    ObjCSynthesizeDecl = 37,
    ObjCDynamicDecl = 38,
    CxxAccessSpecifier = 39,
    FirstDecl = 1,
    LastDecl = 39,
    FirstRef = 40,
    ObjCSuperClassRef = 40,
    ObjCProtocolRef = 41,
    ObjCClassRef = 42,
    TypeRef = 43,
    CxxBaseSpecifier = 44,
    TemplateRef = 45,
    NamespaceRef = 46,
    MemberRef = 47,
    LabelRef = 48,
    OverloadedDeclRef = 49,
    VariableRef = 50,
    LastRef = 50,
    FirstInvalid = 70,
    InvalidFile = 70,
    NoDeclFound = 71,
    NotImplemented = 72,
    InvalidCode = 73,
    LastInvalid = 73,
    FirstExpr = 100,
    UnexposedExpr = 100,
    DeclRefExpr = 101,
    MemberRefExpr = 102,
    CallExpr = 103,
    ObjCMessageExpr = 104,
    BlockExpr = 105,
    IntegerLiteral = 106,
    FloatingLiteral = 107,
    ImaginaryLiteral = 108,
    StringLiteral = 109,
    CharacterLiteral = 110,
    ParenExpr = 111,
    UnaryOperator = 112,
    ArraySubscriptExpr = 113,
    BinaryOperator = 114,
    CompoundAssignOperator = 115,
    ConditionalOperator = 116,
    CStyleCastExpr = 117,
    CompoundLiteralExpr = 118,
    InitListExpr = 119,
    AddrLabelExpr = 120,
    StmtExpr = 121,
    GenericSelectionExpr = 122,
    GnuNullExpr = 123,
    CxxStaticCastExpr = 124,
    CxxDynamicCastExpr = 125,
    CxxReinterpretCastExpr = 126,
    CxxConstCastExpr = 127,
    CxxFunctionalCastExpr = 128,
    CxxTypeidExpr = 129,
    CxxBoolLiteralExpr = 130,
    CxxNullPtrLiteralExpr = 131,
    CxxThisExpr = 132,
    CxxThrowExpr = 133,
    CxxNewExpr = 134,
    CxxDeleteExpr = 135,
    UnaryExpr = 136,
    ObjCStringLiteral = 137,
    ObjCEncodeExpr = 138,
    ObjCSelectorExpr = 139,
    ObjCProtocolExpr = 140,
    ObjCBridgedCastExpr = 141,
    PackExpansionExpr = 142,
    SizeOfPackExpr = 143,
    LambdaExpr = 144,
    ObjCBoolLiteralExpr = 145,
    ObjCSelfExpr = 146,
    ArraySectionExpr = 147,
    ObjCAvailabilityCheckExpr = 148,
    FixedPointLiteral = 149,
    OmpArrayShapingExpr = 150,
    OmpIteratorExpr = 151,
    CxxAddrspaceCastExpr = 152,
    ConceptSpecializationExpr = 153,
    RequiresExpr = 154,
    CxxParenListInitExpr = 155,
    PackIndexingExpr = 156,
    LastExpr = 156,
    FirstStmt = 200,
    UnexposedStmt = 200,
    LabelStmt = 201,
    CompoundStmt = 202,
    CaseStmt = 203,
    DefaultStmt = 204,
    IfStmt = 205,
    SwitchStmt = 206,
    WhileStmt = 207,
    DoStmt = 208,
    ForStmt = 209,
    GotoStmt = 210,
    IndirectGotoStmt = 211,
    ContinueStmt = 212,
    BreakStmt = 213,
    ReturnStmt = 214,
    GccAsmStmt = 215,
    AsmStmt = 215,
    ObjCAtTryStmt = 216,
    ObjCAtCatchStmt = 217,
    ObjCAtFinallyStmt = 218,
    ObjCAtThrowStmt = 219,
    ObjCAtSynchronizedStmt = 220,
    ObjCAutoreleasePoolStmt = 221,
    ObjCForCollectionStmt = 222,
    CxxCatchStmt = 223,
    CxxTryStmt = 224,
    CxxForRangeStmt = 225,
    SehTryStmt = 226,
    SehExceptStmt = 227,
    SehFinallyStmt = 228,
    MsAsmStmt = 229,
    NullStmt = 230,
    DeclStmt = 231,
    OmpParallelDirective = 232,
    OmpSimdDirective = 233,
    OmpForDirective = 234,
    OmpSectionsDirective = 235,
    OmpSectionDirective = 236,
    OmpSingleDirective = 237,
    OmpParallelForDirective = 238,
    OmpParallelSectionsDirective = 239,
    OmpTaskDirective = 240,
    OmpMasterDirective = 241,
    OmpCriticalDirective = 242,
    OmpTaskyieldDirective = 243,
    OmpBarrierDirective = 244,
    OmpTaskwaitDirective = 245,
    OmpFlushDirective = 246,
    SehLeaveStmt = 247,
    OmpOrderedDirective = 248,
    OmpAtomicDirective = 249,
    OmpForSimdDirective = 250,
    OmpParallelForSimdDirective = 251,
    OmpTargetDirective = 252,
    OmpTeamsDirective = 253,
    OmpTaskgroupDirective = 254,
    OmpCancellationPointDirective = 255,
    OmpCancelDirective = 256,
    OmpTargetDataDirective = 257,
    OmpTaskLoopDirective = 258,
    OmpTaskLoopSimdDirective = 259,
    OmpDistributeDirective = 260,
    OmpTargetEnterDataDirective = 261,
    OmpTargetExitDataDirective = 262,
    OmpTargetParallelDirective = 263,
    OmpTargetParallelForDirective = 264,
    OmpTargetUpdateDirective = 265,
    OmpDistributeParallelForDirective = 266,
    OmpDistributeParallelForSimdDirective = 267,
    OmpDistributeSimdDirective = 268,
    OmpTargetParallelForSimdDirective = 269,
    OmpTargetSimdDirective = 270,
    OmpTeamsDistributeDirective = 271,
    OmpTeamsDistributeSimdDirective = 272,
    OmpTeamsDistributeParallelForSimdDirective = 273,
    OmpTeamsDistributeParallelForDirective = 274,
    OmpTargetTeamsDirective = 275,
    OmpTargetTeamsDistributeDirective = 276,
    OmpTargetTeamsDistributeParallelForDirective = 277,
    OmpTargetTeamsDistributeParallelForSimdDirective = 278,
    OmpTargetTeamsDistributeSimdDirective = 279,
    BuiltinBitCastExpr = 280,
    OmpMasterTaskLoopDirective = 281,
    OmpParallelMasterTaskLoopDirective = 282,
    OmpMasterTaskLoopSimdDirective = 283,
    OmpParallelMasterTaskLoopSimdDirective = 284,
    OmpParallelMasterDirective = 285,
    OmpDepobjDirective = 286,
    OmpScanDirective = 287,
    OmpTileDirective = 288,
    OmpCanonicalLoop = 289,
    OmpInteropDirective = 290,
    OmpDispatchDirective = 291,
    OmpMaskedDirective = 292,
    OmpUnrollDirective = 293,
    OmpMetaDirective = 294,
    OmpGenericLoopDirective = 295,
    OmpTeamsGenericLoopDirective = 296,
    OmpTargetTeamsGenericLoopDirective = 297,
    OmpParallelGenericLoopDirective = 298,
    OmpTargetParallelGenericLoopDirective = 299,
    OmpParallelMaskedDirective = 300,
    OmpMaskedTaskLoopDirective = 301,
    OmpMaskedTaskLoopSimdDirective = 302,
    OmpParallelMaskedTaskLoopDirective = 303,
    OmpParallelMaskedTaskLoopSimdDirective = 304,
    OmpErrorDirective = 305,
    OmpScopeDirective = 306,
    OmpReverseDirective = 307,
    OmpInterchangeDirective = 308,
    OpenAccComputeConstruct = 320,
    OpenAccLoopConstruct = 321,
    LastStmt = 321,
    TranslationUnit = 350,
    FirstAttr = 400,
    UnexposedAttr = 400,
    IBActionAttr = 401,
    IBOutletAttr = 402,
    IBOutletCollectionAttr = 403,
    CxxFinalAttr = 404,
    CxxOverrideAttr = 405,
    AnnotateAttr = 406,
    AsmLabelAttr = 407,
    PackedAttr = 408,
    PureAttr = 409,
    ConstAttr = 410,
    NoDuplicateAttr = 411,
    CudaConstantAttr = 412,
    CudaDeviceAttr = 413,
    CudaGlobalAttr = 414,
    CudaHostAttr = 415,
    CudaSharedAttr = 416,
    VisibilityAttr = 417,
    DLLExport = 418,
    DLLImport = 419,
    NSReturnsRetained = 420,
    NSReturnsNotRetained = 421,
    NSReturnsAutoreleased = 422,
    NSConsumesSelf = 423,
    NSConsumed = 424,
    ObjCException = 425,
    ObjCNSObject = 426,
    ObjCIndependentClass = 427,
    ObjCPreciseLifetime = 428,
    ObjCReturnsInnerPointer = 429,
    ObjCRequiresSuper = 430,
    ObjCRootClass = 431,
    ObjCSubclassingRestricted = 432,
    ObjCExplicitProtocolImpl = 433,
    ObjCDesignatedInitializer = 434,
    ObjCRuntimeVisible = 435,
    ObjCBoxable = 436,
    FlagEnum = 437,
    ConvergentAttr = 438,
    WarnUnusedAttr = 439,
    WarnUnusedResultAttr = 440,
    AlignedAttr = 441,
    LastAttr = 441,
    PreprocessingDirective = 500,
    MacroDefinition = 501,
    MacroExpansion = 502,
    MacroInstantiation = 502,
    InclusionDirective = 503,
    FirstPreprocessing = 500,
    LastPreprocessing = 503,
    ModuleImportDecl = 600,
    TypeAliasTemplateDecl = 601,
    StaticAssert = 602,
    FriendDecl = 603,
    ConceptDecl = 604,
    FirstExtraDecl = 600,
    LastExtraDecl = 604,
    OverloadCandidate = 700,
}

// -- public api
