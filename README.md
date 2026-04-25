# enum_handlers

A Rust procedural macro for ergonomic enum variant dispatching. Transform enum match arms routing with type-safe handlers.

## Motivation

When building applications with enums representing messages, events, or commands (for example, Iced), every file has a long `match` boilerplate. `enum_handlers` transforms this pattern into a clean, maintainable dispatch system.

## Benefits

- Boilerplate reduction. Yes it's not idiomatic in Rust, but for larger projects the clean look and clear code intent overweights it
- Zero runtime overhead fully static dispatch
- Easily scalable, just add enum variant and handler
- Full type-safety on argument types and exhaustive handling
- No hidden conventions for handler names
- No configuration other than dispatch function name, everything else is detected from handler functions
- Doesn't break IDE support
- Non-cryptic error messages by compiler

## Quick Start

```rust
use enum_handlers::{with_enum_handlers, handle};

enum Message {
    Add(isize),
    Dec,
}

struct Counter {
    state: isize,
}

#[with_enum_handlers(Message)]
impl Counter {
    #[handle(Message::Add)]
    fn add(&mut self, value: isize) {
        self.state += value;
    }

    #[handle(Message::Dec)]
    fn decrement(&mut self) {
        self.state -= 1;
    }
}
```

## Examples

### 1. Basic Usage

You can easily mix `&self` and `&mut self` handlers. When any handler uses `&mut self`, the dispatch method automatically uses `&mut self`.

```rust
#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Inc)]
    fn handle_inc(&self, Inc(value, times): Inc) { /* reads state */ }

    #[handle(Message::Dec)]
    fn handle_dec(&mut self, value: isize) { /* mutates state */ }
}
```

See [examples/basic.rs](examples/basic.rs)

### 2. Custom Function Name

For example, to use with Iced, standard convention is to name entry point `update`:

```rust
#[with_enum_handlers(Message, dispatch=update)]
impl App {
    // ... handlers ...
    #[handle(Message::TogglePreview)]
    fn on_toggle_preview(&mut self) -> iced::Task {
        self.preview = !self.preview;
        iced::Task::none()
    }
}

// Usage:
iced::application(App::new, App::update, App::view)
```

See [examples/rename.rs](examples/rename.rs)

### 3. Multiple Enums

Handle multiple enums with separate dispatch methods on the same impl block. Each dispatch only processes handlers for its specific enum:

```rust
#[with_enum_handlers(ApiMessage, dispatch=on_api)]
#[with_enum_handlers(BackgroundMessage, dispatch=on_background)]
impl App {
    #[handle(ApiMessage::Inc)]
    fn handle_api_inc(&mut self) { /* API handler */ }

    #[handle(BackgroundMessage::Done)]
    fn handle_done(&mut self, value: isize) { /* Background handler */ }
}

// Usage:
app.on_api(ApiMessage::Inc).await;
app.on_background(BackgroundMessage::Done(5));
```

See [examples/multiple.rs](examples/multiple.rs)

### 4. Immutable Detection

When **all** handlers use `&self`, the dispatch method is generated with `&self`, allowing usage on non-mutable references:

```rust
use std::cell::Cell;

#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Inc)]
    fn handle_inc(&self, Inc(value, times): Inc) -> isize {
        self.state.update(|x| x + value * times);
        self.state.get()
    }
}

// Usage on non-mutable reference:
let app = App { state: Cell::new(0) };
app.dispatch(Message::Inc(Inc(10, 3)));
```

See [examples/immutable.rs](examples/immutable.rs)

### 5. Custom Return Types

Handlers can return any type, but all must be the same (in different `match` arms). The dispatch method uses the return type of the **last** handler encountered in the impl block:

```rust
#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Get)]
    fn handle_get(&self) -> Output { Output(self.state) }

    #[handle(Message::Inc)]
    fn handle_inc(&mut self, value: isize) -> Output { /* ... */ }
}
```

See [examples/return-type.rs](examples/return-type.rs)

### 6. Async Support

Mix sync and async handlers in the same dispatch. When any handler is async, the dispatch method becomes async too:

```rust
#[with_enum_handlers(Message)]
impl App {
    #[handle(Message::Get)]
    fn handle_get(&self) -> isize { self.state }  // sync

    #[handle(Message::Inc)]
    async fn handle_inc(&mut self, value: isize) -> isize { /* async */ }
}

// Usage:
let result = app.dispatch(Message::Inc(10)).await;
```

See [examples/async.rs](examples/async.rs)

## API Reference

### `#[with_enum_handlers(Enum, dispatch = name)]`

Attribute to place on an `impl` block. Generates a dispatch method from `#[handler()]`s that matches on the enum and calls the appropriate handler.

**Parameters:**
- `Enum` — The enum type containing the variants to dispatch
- `dispatch = name` (optional) — Override name for the generated dispatch method (default: `dispatch`)

### `#[handle(Enum::Variant)]`

Attribute to place on individual impl methods. Marks a method as a handler for a specific enum variant. Has no effect without `with_enum_handlers`.

**Parameters:**
- `Enum::Variant` — The enum variant to handle (supports both unit `Msg::Get` and tuple `Msg::Inc(value)` variants)

## How It Works

The macro processes the impl block at compile time:

1. **Collects handlers** — Scans for `#[handle(...)]` attributes and extracts method signatures
2. **Infers signatures** — Automatically determines:
   - Receiver type: `&self` if all handlers are immutable, `&mut self` otherwise
   - Async keyword: `async` if any handler is async
   - Return type: From the last handler in the impl block
3. **Generates dispatch** — Creates a `match` statement routing enum variants to their handlers

## Comparison with Similar Crates

| Feature | `inline_enum_handler` | `enum_handler` / `bloc` |
|---|---|---|
| **Generated code** | Dispatch method directly on `impl` block | Trait with per-variant methods |
| **Handler definition** | Declare methods normally, annotate with `#[handle()]` | Implement generated trait methods for each variant |
| **Configuration per variant** | None — types are auto-detected | Explicit `is_async`, `return_type`, `pass_args_by_ref`, etc. |
| **Trait boilerplate** | Zero — no trait to implement | Split trait implementation and rest of code |

The core difference is that `inline_enum_handler` works inside your `impl` block — you write your handler methods as usual and mark them with `#[handle()]`. No traits, no per-variant configuration. Everything (async, mut/immut, return type) is inferred automatically.

## Constraints

- All handlers for a single enum must have the same return type (inferred from the last handler)
- Static code analysis may struggle with this
