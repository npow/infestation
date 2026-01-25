# Infestation

A top-down 2D grid-based turn-based puzzle game built with macroquad, targeting WASM.

## Design Principles

**Prefer simple, declarative solutions over complex imperative ones.**

- Use existing selectors and patterns instead of manual state management
- Use platform defaults and conventions instead of custom implementations
- If you're tracking state manually, ask if something already tracks it for you
- Fewer lines of code means fewer bugs

**Convention over configuration.**

- Wildcard dependency versions (`*`) during development
- Standard project structure
- Let tools do their jobs with minimal overrides

**Check for existing crates and libraries first.**

- Before implementing ANY utility function, search crates.io for an existing solution
- Common operations (string case conversion, date formatting, etc.) almost always have a crate
- Read the docs. Search the API. Don't assume it's not there.
- If you're about to write infrastructure code, stop and investigate
- "I need to do X" → first search "rust X crate" or check if a dependency already does it
- Hand-rolling utilities is a code smell. If it feels like a solved problem, it probably is.

**When in doubt, delete code.**

- The best code is no code
- If the framework provides it, don't reimplement it
- Don't hand-roll something unnecessarily complicated when a simple solution exists

**Don't repeat yourself.**

- If you're doing the same thing in multiple places, use the same code
- Factor out common logic before special cases
- Duplication is a sign that structure is missing

**No stringly-typed programming.**

- Don't use strings as stand-ins for structured data
- Use enums, newtypes, or Option instead of magic string values
- `Option<T>` is better than a sentinel value

**Use structs instead of large tuples.**

- Tuples with 3+ elements become hard to read and easy to misuse
- Named fields are self-documenting; `.2` is not
- A struct makes destructuring patterns clearer and refactoring safer

**Preserve error information.**

- Don't discard the original error with `.map_err(|_| ...)` or `.ok()`
- Wrap errors to add context, don't replace them
- Error messages should help diagnose the problem, not hide it

**Never leak memory.**

- Do not use `Box::leak`, `mem::forget`, or similar without explicit approval
- If you think leaking is the right solution, ask first - you're probably wrong

**Prefer `?` over `map` for Option and Result.**

- Use `?` for early returns instead of chaining `.map()` or `.and_then()`
- Exception: eta-reducible cases like `.map(f)` are fine
- `let x = foo()?; bar(x)` is clearer than `foo().map(|x| bar(x))`

**Test behavior, not implementation.**

- Tests should verify observable outcomes (grid state, play state, entity positions)
- Don't assert on internal variables or intermediate state (e.g., `anim.triggered_numbers`)
- If the implementation changes but behavior stays the same, tests should still pass
- A good test describes what the game does, not how it does it

**Use Position and PositionDelta operations, not raw x/y.**

- Use `pos + delta` instead of `Position { x: pos.x + delta.dx, y: pos.y + delta.dy }`
- Use `pos1 - pos2` to get a `PositionDelta` instead of `PositionDelta::new(pos1.x - pos2.x, ...)`
- Use `pos.in_bounds(bounds)` instead of manual comparisons with width/height
- Keep position arithmetic at the type level; only access `.x`/`.y`/`.dx`/`.dy` for final output (e.g., rendering)
