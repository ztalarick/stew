# MVS

What I want to initialy build.


## basic principles

- no magic
- safety first
- first class functions / functional
- everything is an expression
- micro imperative

## General

- Everything lives in a single file, single global namespace
- The last expression in a block (without trailing ;) is the block's value. A trailing ; makes the block evaluate to (). An if used as an expression must have an else.

## Native Types

- int (default machine dependent)
    - u8
    - i64
- float
    - f64
- bool
- char
    - unicode
    - 32 bits
- str
    - Native UTF-8 string slice (pointer + length)
    - In v0, all strings are literals stored in the binary's static segment
    - Freely copyable (copies the pointer, not the bytes)
    - Dynamic string construction (concatenation, formatting) deferred to a future String
- array
    - not dynamic
    - [T; N]
- function
- struct
- tuple
    - nameless struct
- unit
    - empty type
- choice 
    - Algebraic Data Type (ADT) that can hold data
    - choice Option { Some(int), None }
- reference
    - &T (Immutable, Ephemeral Arena)
    - &mut T (Mutable, Ephemeral Arena)
    - shared &T 
- slice
    - &[T]
    - &mut [T]

## Keywords

- **mut**
    - data is immutable by default
    - declares a variable mutable
    ```rust
    let mut x: int = 5
    x = x + 1
    ```
    - not allowed to be declared globally
    - cannot be stored inside a `shared T`
    - can never be placed into any final State struct (anything global)
    - can never be sent across a channel or thread

- **shared**
    - Immutable
    - Reference-Counted
    - Survives Arenas
    - must be a reference

- loops
    - **break** (evals to ())
    - **continue** (evals to ())
    - **in** (iterator) (also used in machine context)
    - **for**
    - **while**

- **fun** (declares a function)
    - **return** (returns from function)

- **machine** (declares new machine)
    - **event** (declares event machine can receive)
    - **state** (declares a possible state a machine is in and its schema)
    - **in** (declaring the code for when a machine is in a certain space)
    - **on** (declaring code to run when receiving that event, always evals to expression of the machine type)

- **if**
- **else**

- `and`
- `or`
- `not`

## Operators

- Math
    - `+`
    - `-`
    - `/`
    - `*`
    - `%` (modulo)
    - ( ) order of operations also applies to bool operations

- Boolean operations
    - `and`
    - `or`
    - `not`

- comparison
    - `==`
    - `!=`
    - `<`
    - `>`
    - `<=`
    - `>=`

- Bit shifting (needed?)
    - `>>`
    - `<<`

- Bitwise
    - `&`
    - `|`
    - `^` (XOR)

- `=` for assignment and reassignment

## Puncuation

- ;
    - rust style
    - if ends in ; discard (eval to unit)
    - without evals to the expression
    - function evals to last expresion

## Arrays

```rust
let numbers: [int; 4] = [10, 20, 30, 40];

let fruits: [str; 3] = ["apple", "banana", "cherry"];

// Direct Indexing
let first: str = fruits[0]; // "apple"

// just a snippet, can't have global mutable variables
let mut mutable_arr: [int; 3] = [1, 2, 3];
mutable_arr[0] = 99; // Allowed because of 'mut'

// Iterating over elements
for value in mutable_arr {
    // ...
}
```

## Structs

```rust
struct UserData {
    id: int
    name: str
}

// explicit var name based
let henry: UserData = {id: 0, name: "henry"};

// field access
let henrys_name: str = henry.name;

```

## Tuples

```rust
let henry: (int, str) = (0, "henry");

// field access
let henrys_name: str = henry.1;
```

## Choice

```rust
choice StopLight {
    | Red
    | Yellow
    | Green
}

let color: Light = Light.Red;

choice OptionInt {
    | Some(int)
    | None
}

let num: OptionInt = OptionInt.Some(3);
```


## Memory

Arena based memory allocation. Once the arena is completed everything is destroyed. Mutable data is not allowed to leave an arena. Compiler enforces strict boundaries on mutable data. No borrow checking needed.

- No global arenas
- arenas are thread/event specific
- global variables are static memory

Rules:
1. Arenas are single-owner, single-thread — no concurrent access to mutable data within an arena
2. Mutable data cannot escape its arena — the compiler enforces this at the boundary
3. When the arena dies, everything in it dies — no individual frees, no dangling pointers possible


### Allowed

**Inside Functions**
```rust
// Mutable variable never leaves the function. Safe
fun calculate_sum(numbers: &[i64]) -> i64 {
    let mut total: i64 = 0; // Local mutation
    for n in numbers {
        total = total + n;
    }
    return total; // Returns an immutable value
}
```

**Passing Mutable References**
```rust
// The function takes a reference, modifies it, and returns a value. Safe.
fun update_user_score(user: &mut User, points: i64) -> () {
    user.score = user.score + points; // Mutating the data owned by the caller
}
```

**Reference derived from input**
```rust
// The function takes a reference and returns a reference derived from the input. Safe.
fun get_first(items: &[i64]) -> &i64 {
    return &items[0]; 
}
```

### Not Allowed

**Returning a Reference to Local Data (Dangling Pointers)**
```rust
fun create_dangling_pointer() -> &i64 {
    let mut x = 100; // x is created in the local context
    return &x;       // ERROR: You cannot return a pointer to a local variable.
}
```


## Control Flow

- starts in main function

```rust
// can also return int for exit codes
// args automatically mapped to string slice
// unit () is successful exit
fun main(args: &[str]) -> () {
    // begin program execution
} 
```

- match
    - ocaml style
    - enforces exhaustive pattern matching on choice 
    - wildcard is **not** allowed when using a choice
```rust
choice Light {
    | Red
    | Yellow
    | Green
}

match light_change {
    | Light.Red -> stoplight.set_color(red)
    | Light.Yellow -> stoplight.set_color(yellow)
    | Light.Green -> stoplight.set_color(green)
}

// also works on non choice
let money: int = 100;
let description: str = match money {
    | x if x == 0 -> "you are broke"
    | x if x > 10 -> "you are rich!"
    | _ -> "keep earning money"
};
```

- if/else
```rust
// if condition { ... } else { ... }
// condition -> expression which must evaluate to a bool!
// no truthy nonsense
let should_be_six: bool = true;

let num: int = if should_be_six {
    6
} else {
    7
}

// else if
let should_be_five: bool = false;
let n: int = if should_be_five {
    5
} else if should_be_six {
    6
} else {
    7
};

// if used as an expresion, must have an else!
// only if evals to ()
```

- functions

```rust
fun add(x: int, y: int) -> int {
    x + y
}

// Return statements leave early
// in the future this will be Result<f64, E>
fun bad_divide (x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return 0.0;
    }
    x / y
}

// anonymous
let my_add: (int, int) -> int = (x: int, y: int) -> x + y;
let four: int = my_add(2, 2);

// not allowed to capture mutable values in anonymous functions
// let mut base_value = 10; Not allowed!
// Read only references allowd (&T, &[T], shared T)
let base_value: int = 10;
let add_to_base = (x: int) -> base_value + x;

// no return and last statement is semicolon returns unit type ()
fun do_something() -> () {
    something();
}

```

- for loops

```rust
// in syntax for any iterator
let flavors: [str; 3] = ["cherry", "blueberry", "grape"];
for flavor: str in flavors {
    if flavor == "cherry" {
        eat()
    } else {
        toss()
    }
}

// no number based `for`s, use while for crazy stuff
// evaluates to ()

let price_items: [(str, int); 3] = [("pogo_stick", 15), ("roller_skates", 30), ("keyboard", 50)];

for item: str, price: int in price_items {
    print(item);
    print(price);
}
```

- while loops

```rust
// while condition { ... }
// condition -> expression which must evaluate to a bool
let mut x: int = 100
while x > 0 {
    x = x - 5;
}

// evaluates to ()

let mut x: int = 100
while x > 0 {
    if is_even(x) {
        break;
    } else {
        x = x - 5;
        continue;
    }
}
```

- state machines
    - delcare intial state = .init()
    - declare possible states
    - declare transitions on each state
    - declare the data schema for each state
        - can be different in any state
    - declare the event payloads schema
    - read via choice match expression on possible states
    - return type for send = reference to the machine
    - mutate data by remaining in the same state
    - send events with .send()
    - all states must handle all events (can use wildcard on _ {})

```rust
// A pure helper function to calculate tax
fun calculate_tax(amount: int) -> int {
    let tax_rate: int = 5; // 5 percent
    return (amount * tax_rate) / 100;
}

// A pure helper function to generate a string
fun generate_receipt(cart_id: int) -> str {
    return "RECEIPT-12345";
}

machine ShoppingCart {
    
    // Natively defined events
    event AddItem(price: int)
    event StartCheckout
    event ConfirmPayment
    event DeleteCart

    // Natively defined states (Notice the different schemas!)
    state Browsing(cart_id: int, item_count: int, subtotal: int)
    state CheckingOut(cart_id: int, final_total: int)
    state Paid(receipt_id: str, amount_paid: int)
    state Terminated

    // Spawns the machine. We can pass arguments into init!
    init(id: int) -> ShoppingCart {
        Browsing(id, 0, 0)
    }

    // STATE: BROWSING
    in Browsing(cart_id: int, item_count: int, subtotal: int) {
        
        on AddItem(price: int) -> ShoppingCart {
            let mut new_count: int = item_count + 1;
            let mut new_subtotal: int = subtotal + price;
            
            // Stays in the same state, compiler infers an update, keeps the arena
            Browsing(cart_id, new_count, new_subtotal)
        }

        on StartCheckout -> ShoppingCart {
            // Call our external helper function!
            let tax: int = calculate_tax(subtotal);
            let final_total: int = subtotal + tax;
            
            // Transition to the Checkout state (Drops the old arena!)
            CheckingOut(cart_id, final_total)
        }

        on DeleteCart -> ShoppingCart {
            // Needs explicit Terminated state
            Terminated
        }

        on ConfirmPayment -> ShoppingCart {
            // Cannot pay while browsing! Ignore the event.
            Browsing(cart_id, item_count, subtotal)
        }
    }

    // STATE: CHECKING OUT
    in CheckingOut(cart_id: int, final_total: int) {
        
        on ConfirmPayment -> ShoppingCart {
            let receipt: str = generate_receipt(cart_id);
            Paid(receipt, final_total)
        }

        on AddItem(price: int) -> ShoppingCart {
            // If they add an item during checkout, we reject the state 
            // and throw them back to Browsing!
            Browsing(cart_id, 1, price)
        }

        on DeleteCart -> ShoppingCart {
            Terminated
        }

        on _ -> ShoppingCart{
            CheckingOut(cart_id, final_total)
        }
    }

    // STATE: PAID
    in Paid(receipt_id: str, amount_paid: int) {
        
        on DeleteCart -> ShoppingCart {
            Terminated
        }

        on _ -> ShoppingCart {
            // Once paid, no other events do anything. 
            Paid(receipt_id, amount_paid)
        }
    }
    
    // STATE: TERMINATED
    // User must define this if they want a terminated state
    // Eats all events
    // Forces callers to handle possibility of terminated machines.
    in Terminated {
        on _ -> ShoppingCart {
            Terminated
        }
    }
}

fun handle_user_request() -> () {
    
    // 1. Spawn a new cart with ID 99
    // In v0, ShoppingCart.init() returns an owned value that lives in the caller's arena. .send() mutates the machine in- place (requires mut). .state returns the current state for pattern matching. Machines are not copyable.
    let mut cart: ShoppingCart = ShoppingCart.init(99);

    // 2. Send some events
    cart.send(ShoppingCart.AddItem(500));
    cart.send(ShoppingCart.StartCheckout);
    cart.send(ShoppingCart.ConfirmPayment);

    // 3. SAFELY READ THE STATE
    // We want to print the receipt, but what if the payment failed?
    // The compiler forces us to check the schema via pattern matching!
    // ADT of machine state automatically generated by compiler for reads
    match cart.state {   
        | ShoppingCart.Browsing(id, count, total) -> {
            "Cart is still browsing."
          }
        | ShoppingCart.CheckingOut(id, total) -> {
            "Cart is stuck in checkout."
          }
        | ShoppingCart.Paid(receipt, total) -> {
            // We ONLY have access to the receipt string inside this block.
            // Absolute type safety!
            receipt
          }
        | ShoppingCart.Terminated -> {
            "Cart is deleted"
        }
    }
    
    // 4. Clean up
    cart.send(ShoppingCart.DeleteCart);
}
```

## Print

Compiler intrinsic. Magic for v0

```rust
// The compiler magically accepts any native primitive:
print("Hello");
print(42);
print(true);
```


## Future/Nice to haves

- type inference
```
let x = 4
// `x: int` type is inferred
```
- some sort of range syntax
```rust
for i in range 1..10 {
    //...
}
```
- python style set math (I really love this from python, but maybe its too magic)
- never types for break / continue
```
// If the user isn't found, we `continue`. 
// Because `continue` never finishes evaluating, the compiler allows it to sit where an `int` is expected!
let score: int = if user.is_active { user.score } else { continue };

// For break:
// Because a loop is an expression, you can actually allow break to return a value from the loop.

let mut x: int = 0;

// The loop itself evaluates to an int!
let found_value: int = while x < 100 {
    if is_target(x) {
        break x; // This causes the entire while loop to evaluate to `x`
    }
    x = x + 1;
};
```
- generics
- python list comprehension? maybe doesn't make sense for native arrays
- need some way for comprehension (go interfaces? rust traits?)
- modules rust style for encapsulation
    - this one is very important to me, consider adding to v0