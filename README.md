# ansi_to_appcui

Converts an ANSI string into AppCUI [`Character`](https://docs.rs/appcui/latest/appcui/graphics/struct.Character.html) values.

SGR sequences set the colors and style of the following characters: the 16 ANSI colors, 256-color and 24-bit colors, bold, italic, underline (single, double, curly, dotted), strike-through, and reverse video. Cursor movement, erase, and other non-SGR sequences are skipped. The style starts as the parser's default attribute (white on black) and SGR reset returns to it.

```rust
use ansi_to_appcui::{AnsiParserBuilder, OnError};
use appcui::prelude::*;

let mut parser = AnsiParserBuilder::new().build();
let mut chars = Vec::new();
parser.write_into_vec("\x1b[1;31mHi\x1b[0m", &mut chars);
```

`on_error` controls malformed sequences:

- `OnError::Ignore` (default) drops them and continues
- `OnError::Collect` records them in `parser.errors()` and continues
- `OnError::Stop` records the first one and stops; characters already written stay in the vector

```rust
let mut parser = AnsiParserBuilder::new().on_error(OnError::Collect).build();
```
