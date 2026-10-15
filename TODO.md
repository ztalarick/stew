# stew-core

[x] - Core AST
[] - Type checking and semantic analysis
[] - Interpreter

# stew-parser

[x] - Translate tree-sitter CST into Core AST

# stew-cli

[] - Ideally should move the error reporting outside of parser and core. Onlt this crate should have miette

# tree-sitter-stew

[] - Test cases of parse tree

  create a test/corpus/ directory
  Create a test file like `test/corpus/hello_world.stew`.

    ```
    ==================
    Basic Variable Declaration
    ==================
    let mut x: int = 5;
    ---
    (source_file
        (let_declaration
        (identifier_type_annotation
            (identifier)
            (type))
        (int_literal)))
    ```
    ```bash
    tree-sitter test
    ```

# other

[] - Language Server
[] - Syntax Highlighter vscode plugin


# Roadmap

---

```
                       CURRENT STATE
                [Tree-sitter CST]  ──►  [Stew AST]
                                           │
                                           ▼
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 1: Semantic Analysis & Type Checking (Cooper & Torczon Ch 5 & 6)  │
│          • Name Resolution (Symbol Table)                              │
│          • Type Checking & Inference                                   │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 2: Design the Linear CFG IR Data Structures                      │
│          • BasicBlock, Instruction, Terminator                         │
│          • Human-readable IR Pretty-Printer (for debugging)            │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 3: Lowering: AST ──► CFG IR (Flattening)                         │
│          • Expressions & Variables (Alloca/Load/Store)                 │
│          • Control Flow (If, While, Match)                             │
│          • Stew Machines (State transitions to jumps)                  │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 4: The CFG Interpreter (`stew run`)                              │
│          • Stack frames & instruction execution loop                   │
│          • Language verification test suite                            │
└──────────────────────────────────┬─────────────────────────────────────┘
                                   ▼
┌────────────────────────────────────────────────────────────────────────┐
│ PHASE 5: Code Generation (LLVM)                                        │
│          • 1:1 translation of CFG Blocks to LLVM IR (via Inkwell)      │
│          • JIT or AOT native compilation                               │
└────────────────────────────────────────────────────────────────────────┘
```

---

### Phase 1: Semantic Analysis & Type Checking
*Goal: Ensure the program is semantically valid before attempting to lower it.*

1. **Milestone 1.1: The Symbol Table (Scope Management)**
   * Create a `SymbolTable` struct (a stack of scopes, e.g., `Vec<HashMap<String, SymbolInfo>>`).
   * When entering a block or function: push a new scope.
   * When exiting: pop the scope.
   * Verify that variables are declared before use and catch shadowing/scoping bugs.

2. **Milestone 1.2: Type Checker**
   * Walk your [`ast::Program`](file:///Users/ztalarick/Documents/workspace/state/stew/crates/stew-core/src/ast.rs#L46) to compute and verify types:
     * In `a + b`: Verify both sides are numeric types.
     * In `let x: int = expr`: Verify the type of `expr` matches `int`.
     * In functions: Verify return expressions match the function's [`return_type`](file:///Users/ztalarick/Documents/workspace/state/stew/crates/stew-core/src/ast.rs#L59).
   * Emit clear errors using `miette` when types don't match.

---

### Phase 2: Design the Linear CFG IR Data Structures
*Goal: Define the target representation where trees become flat blocks.*

1. **Milestone 2.1: Define the IR Structures**
   * **`BasicBlock`**: Has a unique `BlockId`, a `Vec<Instruction>`, and exactly one `Terminator`.
   * **`Instruction`** (Non-branching):
     * `Alloca { target: Reg, ty: Type }` (allocates local variable space on the stack).
     * `Store { ptr: Reg, value: Operand }`
     * `Load { target: Reg, ptr: Reg }`
     * `Binary { target: Reg, op: BinaryOp, left: Operand, right: Operand }`
     * `Call { target: Option<Reg>, func: String, args: Vec<Operand> }`
   * **`Terminator`** (Control flow):
     * `Jump(BlockId)`
     * `BranchIf { cond: Operand, then_block: BlockId, else_block: BlockId }`
     * `Return(Option<Operand>)`

2. **Milestone 2.2: The IR Pretty-Printer (Crucial!)**
   * Write a function to print your IR in readable text (e.g., `bb0: v1 = add v0, 2; br v1, bb1, bb2`).
   * *Why this is essential:* You cannot debug an IR you cannot see.

---

### Phase 3: Lowering: AST $\to$ CFG IR (Flattening Control Flow)
*Goal: Translate recursive AST trees into basic blocks.*

Implement this incrementally in stages:

1. **Stage 1: Straight-Line Code**
   * Lower simple functions containing only `let`, assignments, and arithmetic.
   * Every variable becomes an `Alloca`, followed by `Store` and `Load`.

2. **Stage 2: Branches and Loops**
   * **`If / Else`**: 
     * Create 3 new blocks: `then_block`, `else_block`, and `merge_block`.
     * End the current block with `BranchIf(cond, then_block, else_block)`.
     * Have both `then` and `else` jump to `merge_block`.
   * **`While`**:
     * Create `cond_block`, `body_block`, and `exit_block`.

3. **Stage 3: Stew-Specific Constructs**
   * Lower [`ast::Statement::Machine`](file:///Users/ztalarick/Documents/workspace/state/stew/crates/stew-core/src/ast.rs#L89-L103):
     * Represent the machine's current state as an integer/enum variable.
     * Lower event handlers into a dispatch block (a jump table or chained `BranchIf`s based on `(state, event)`).

---

### Phase 4: Build the CFG Interpreter
*Goal: Bring `stew run` in [`main.rs`](file:///Users/ztalarick/Documents/workspace/state/stew/crates/stew-cli/src/main.rs#L54) to life.*

1. **Milestone 4.1: Interpreter State**
   * **`StackFrame`**: Holds a map of virtual registers/stack slots to runtime values (`Value::Int(i64)`, etc.).
   * **`CallStack`**: A stack of `StackFrame`s for function calls.

2. **Milestone 4.2: Execution Loop**
   * Maintain `current_block_id`.
   * Loop through each `Instruction` in that block, updating the stack frame.
   * Inspect the `Terminator`:
     * If `Jump(id)` $\to$ set `current_block_id = id`.
     * If `BranchIf(...)` $\to$ choose next block based on condition.
     * If `Return(...)` $\to$ pop stack frame and return value.

3. **Milestone 4.3: Test Language Features**
   * Write Stew test scripts to verify recursion, loops, mutations, and state machine transitions.

---

### Phase 5: Code Generation to LLVM
*Goal: Native compilation.*

Once the interpreter runs your CFG IR:
1. Use the [`inkwell`](https://github.com/TheDan64/inkwell) crate (safe Rust wrapper for LLVM).
2. For each `BasicBlock` in your IR, call `context.append_basic_block(...)`.
3. For each `Instruction`, call the matching LLVM builder function (e.g., `builder.build_int_add(...)`).
4. Run LLVM's `mem2reg` optimization pass: it will automatically convert all your `alloca/load/store` operations into fast SSA registers with $\phi$-nodes.

---

### Immediate Next Action
Start with **Phase 1 (Semantic Analysis / Symbol Table)**. In *Engineering a Compiler*, this matches the middle of Chapter 5 and Chapter 6 (Type Systems and Symbol Tables).