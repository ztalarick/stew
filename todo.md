# Grammar

### 1. Variables & Assignment

[x] Mutability: Add support for the mut keyword in let declarations (let mut x: int = 5;)
[x] Assignment Expression: Reassigning variables (x = 6;)
[x] Array Literals: [10, 20, 30]
[x] Struct Literals: {id: 0, name: "henry"}
[x] Tuple Literals: (0, "henry")

### 2. Operators & Expressions

You will need to use prec.left heavily here to handle order of operations!

[x] Math Operators: +, -, *, /, %
[x] Boolean Operators: and, or, not
[x] Comparison Operators: ==, !=, <, >, <=, >=
[x] Bitwise/Shift Operators: &, |, <<, >>
[x] Struct Field Access: henry.name or tuple access henry.1
[x] Array Indexing: fruits[0]

### 3. Types (Expanding your $.type rule)

[x] Arrays: [int; 4]
[x] Tuples: (int, str)
[x] References: &int, &mut int, shared &int
[x] Slices: &[int], &mut [int]
[x] Function Types: (int, int) -> int

### 4. Control Flow

[x] Blocks: Properly parse { ... } which contains statements and an optional final expression without a
semicolon.
[x] If / Else: if condition { ... } else { ... }
[x] While Loops: while condition { ... }
[x] For Loops: for flavor in flavors { ... }
[x] Loop Controls: break and continue
[x] Return statements
[x] Match Expressions:
    • match value { | Pattern -> Block, | _ -> Block }
    • Guard clauses: | x if x > 10 -> Block


### 5. Advanced Definitions

[x] Structs: struct UserData { id: int, name: str }
[x] Choices (ADTs): choice OptionInt { | Some(int) | None }
[x] Anonymous Functions: (x: int, y: int) -> x + y

### 6. The machine Block

[x] machine declaration
[x] event definitions
[x] state data schemas
[x] init function blocks
[x] in State { ... } context blocks
[x] on Event -> State { ... } transition blocks