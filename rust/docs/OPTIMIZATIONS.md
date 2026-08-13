# Rust Idioms — Open Optimizations

Findings from applying the `rust-idioms` skill checklist (`~/.claude/skills/rust-idioms/`) to `src/main.rs`. Cross-reference: [`../CLAUDE.md`](../CLAUDE.md) for project conventions (hand-roll over crates, book structure preserved, ugly-then-refactor).

Convention: resolved items stay listed, moved to **Resolved** — not deleted. Each open item cites the skill category it falls under so the reasoning is traceable back to `references/*.md` in the skill.

## Resolved

### `resolve_entities` — quadratic allocation + magic-number advance
**Was:** `chars: Vec<char> = text.chars().collect()` upfront, then `chars[i..].iter().collect::<String>()` reallocated on every `&` encountered (worst case approaches O(n²) on entity-dense input), with `i += 4` duplicating the pattern length as a bare literal in three places per branch.
**Now:** `remainder: &str` reslices the original borrowed text (zero allocation during the scan); `strip_prefix` ties "matched" and "advance amount" into one operation. Manual scan-and-advance shape kept (matches book structure per project convention) — only the allocation/indexing internals changed.
Category: *ownership-and-performance* (allocation discipline), *antipatterns* (unwrap → `.expect("reason")`).

### Silent `unwrap_or` fallbacks on parse failures (old item #1)
**Was:** `p.parse::<u16>().unwrap_or(80)` for a URL port and `value.trim().parse().unwrap_or(0)` for the `Content-Length` header — both treated "failed to parse" as "parsed to a specific fallback value," making malformed input indistinguishable from valid data of a different meaning.
**Now:** both call sites propagate instead — `main.rs:417` (port) and `main.rs:537` (Content-Length) use `.map_err(std::io::Error::other)?`. `Url::new` was widened to `-> std::io::Result<Self>` to carry the port-parse error out. Fallout: this signature change broke `tests.rs` until fixed (see **`tests.rs` compile break**, below).
Category: *errors* ("`unwrap_or` as silent fallback" — same smell as bare `.unwrap()`).

### `tests.rs` didn't compile against `Url::new`'s new `Result` signature
**Was:** four tests (`test_url_parses_http`, `_https`, `_path`, `_file_scheme`, `src/tests.rs:18-45`) called `Url::new(...)` and read `.scheme`/`.host`/`.path` straight off the result — written against the pre-fix `Url::new(&str) -> Self`. After the item above widened it to `-> std::io::Result<Self>`, every direct field access failed with `E0609` (`no field 'scheme' on type 'Result<Url, io::Error>'`) — 12 errors, `cargo check --tests` / `cargo test` / `cargo clippy --all-targets` all red, while plain `cargo check` (bin only) stayed green and gave no warning anything was wrong.
**Now:** each of the four call sites got `.unwrap()` appended (`Url::new("...").unwrap()`) — matches this repo's own convention that `.unwrap()` is fine in test code. `cargo test` — 13 passed, `cargo clippy --all-targets` clean.
Category: not a `rust-idioms` finding on its own — direct fallout of the item above. Worth remembering: a signature change that only breaks `--tests` is invisible to a bare `cargo check` inner loop.

## Open

### 2. `Url::new` — name still says infallible, `Result` in the signature says otherwise
`src/main.rs:413` — `fn new(input: &str) -> std::io::Result<Self>`. Partial fix landed since this item was first written: the port-parse fallback it originally flagged (see **Resolved**) is gone, malformed ports now surface as `Err`. What's left: per API-design convention, `new` implies infallible; a `Result`-returning constructor reads more idiomatically as `parse`/`TryFrom` (`Url::parse(&str) -> Result<Self, ...>` or `impl TryFrom<&str> for Url`), so the fallibility is visible at the call site without opening the function body. One silent-default path also survives: `input.split_once("://").unwrap_or(("", input))` still treats a missing `://` as `scheme: ""` instead of erroring — `Url::new("not a url")` parses instead of failing, same shape as the now-fixed port case.
Category: *type-driven-design* ("parse, don't validate" — naming) + *errors* (remaining silent scheme fallback).
Note: this is the same idiom already correctly reached for in `resolve_entities`'s neighbor code — not proposing new machinery, just naming/shape consistency.

### 3. Connection cache keyed on hostname alone
`src/main.rs:17` (`connection_cache: HashMap<String, BufReader<NetworkStream>>`), used at `:465` and `:604`. Already flagged in `CLAUDE.md`'s Known Limitations as a behavior gap (wrong scheme/port reuses a cached connection); from the idioms side, the fix shape is a small `Eq + Hash` key struct (`ConnKey { scheme, host, port }`) instead of widening string formatting — turns "forgot to key on port" into a compile-time-checked field, not a runtime bug someone has to remember to avoid.
Category: *type-driven-design* ("make illegal states unrepresentable" applied to a cache key, not just a domain value).

### 4. Full `InputState` clone every redraw frame
`src/main.rs:142` — `let input = ctx.input(|i| i.clone());` clones the entire `egui::InputState` (events, pointer state, etc.) every frame just to read four `key_pressed` booleans. `egui::Context::input` exists precisely so the closure can read without cloning — the four checks below (`:143-154`) can move directly inside `ctx.input(|i| ...)` and return the composed `scroll_delta`, no clone needed.
Category: *ownership-and-performance* ("don't allocate to satisfy a signature" — here, don't clone to satisfy convenience).

### 5. `Layout::flush` recomputes `row_height` in three separate passes over the same line
`src/main.rs:276-297` — `max_ascent`, then `max_descent`, then the final placement loop each call `ctx.fonts_mut(|f| f.row_height(&font_id_for(*b, *i, *s)))` independently per word, so every word's row height is computed up to 3×. `font_id_for` is cheap, but `ctx.fonts_mut` takes an internal lock/borrow each call — three lock round-trips per word where one pass computing `(ascent, descent)` together and reusing it in the placement loop would do.
Category: *ownership-and-performance* ("measure, don't guess" — flagging as worth a `criterion` check before/after, since egui's internal locking cost is the real unknown here, not the arithmetic).

### 6. `(f32, String, bool, bool, f32)` line-buffer tuple — two adjacent `bool`s
`src/main.rs:230` — `line: Vec<(f32, String, bool, bool, f32)>`, already annotated in-code as `(x, word, bold, italic, size)`. This is exactly the project's own stated threshold ("no helper struct until a tuple gets unwieldy — 5+ fields, used in 3+ places" — `CLAUDE.md`) hit: 5 fields, read/written in `word()` (`:253-265`) and `flush()` (`:270-302`). The two adjacent `bool`s are also the specific transposition risk called out in type-driven-design (swap `bold`/`italic` at a call site, compiler says nothing). A small `LineItem { x, word, bold, italic, size }` struct removes both the field-count and the boolean-position risk.
Category: *type-driven-design* ("enums/structs over positional bool params") + project's own tuple-size convention.

### 7. `strip_tags` clones the buffer instead of taking it
`src/main.rs:616` and `:623` — `out.push(HtmlBody::Text(buffer.clone())); buffer.clear();` (and the `Tag` equivalent). The clone is immediately followed by clearing the original, i.e. the old contents are never read again after the push — a textbook `std::mem::take(&mut buffer)` (moves the `String` out, leaves an empty one behind, no copy) replaces both lines with one, no allocation.
Category: *antipatterns* ("too many clones" — reflexive clone where the value could simply move).

### 8. `install_fonts` — five near-identical `font_data.insert` blocks
`src/main.rs:58-91` repeats the same three-line shape five times (name, path, `Arc::new(FontData::from_static(include_bytes!(...)))`). The font-*families* loop three lines below it (`:101-110`) already demonstrates the data-driven fix: an array of `(name, bytes)` iterated with `for`. Same treatment collapses the five insert blocks to one loop body, one place to touch when a font is added or removed.
Category: *ownership-and-performance* ("iterators over repeated hand-unrolled code") + *api-design* ("do one thing well" — dedup, not new abstraction).

### 9. Minor: `to_string()` vs `.to_owned()` inconsistency
`src/main.rs:424-425` — `scheme.to_string()`, `host.to_string()` on `&str`, while the rest of the file (`:23`, elsewhere) consistently uses `.to_owned()` for the same `&str → String` conversion. No behavior difference (both go through the same `ToOwned`/`ToString` path for `str`), pure consistency nit worth a pass since it's already the codebase's chosen idiom.
Category: *api-design* ("unsurprising" — one convention, not two, for the same operation).

### 10. Minor: `self.url.clone()` repeated at three call sites
`src/main.rs:34`, `:127`, `:133` — each clones `self.url` before calling `self.navigate(&url)`, because `navigate` needs `&mut self` while also reading `self.url`. This is a legitimate borrow-checker exit (duplicate the value — see *ownership-and-performance*), not a bug, but it's the same clone written three times. Low priority; only worth touching if `navigate` is refactored anyway (e.g. to take an owned `String` and read `self.url` before the mutable borrow starts, or via `std::mem::take`).

### 18. Chunked body reads and discards the trailing CRLF without validating it
`src/main.rs:587-588` — inside the `BodyEncoding::Chunked` loop, `reader.read_line(&mut crlf)?` reads the mandatory CRLF that follows each chunk's data but never checks it's actually `"\r\n"`. A server sending malformed chunk framing (wrong chunk-size, missing terminator) won't error here — the corruption only surfaces later as a confusing failure when the next chunk-size line fails to parse as hex, several lines removed from its real cause.
Category: *errors* (validate protocol invariants at the point they're read, not several reads downstream — same shape as the resolved `unwrap_or` items, applied to a framing byte instead of a header value).

## Clippy pedantic (editor-level lints, not in this repo's plain `cargo clippy`)

Repo's own `cargo clippy` (no `clippy.toml`, no lint config in `Cargo.toml`) is clean. These came from an editor running a stricter set (`pedantic` + `-D clippy::unwrap-used` + `-W clippy::arithmetic-side-effects`). Listed because they land on lines already touched above or catch things the idioms pass above missed — not because the repo currently enforces them.

### 11. `unwrap()` on `Option` — `install_fonts`
`src/main.rs:96` — `.get_mut(&egui::FontFamily::Proportional).unwrap()`. `egui::FontDefinitions::default()` always populates `Proportional`, so the panic is unreachable in practice, but a bare `.unwrap()` doesn't say that — `.expect("egui always populates the Proportional family by default")` documents the assumption instead of leaving `called Option::unwrap() on a None value` as the only clue if egui ever changes that default.
Category: *errors* / *antipatterns* ("`unwrap()` as a review flag").

### 12. Missing `# Panics` doc — `install_fonts`
`src/main.rs:55` — `pub fn install_fonts` can panic (via #11) but has no `# Panics` section. Same root cause as #11: either document it, or remove the panic path by handling the `None` case (`if let Some(proportional) = ... { ... }`, silently no-op if egui's default ever changes shape) and the doc requirement disappears with it.
Category: *api-design* ("document every panic + every error condition").

### 13. Redundant closure — `ctx.input(|i| i.clone())`
`src/main.rs:142` — same line as open item #4. Clippy's mechanical fix is `ctx.input(Clone::clone)` (pass the method itself instead of wrapping it in a closure) — but that still clones the whole `InputState`. Fixing #4 properly (read the four `key_pressed` bools inside the closure, no clone at all) resolves this lint for free, so do #4, not the mechanical swap.
Category: *ownership-and-performance* (duplicate of #4's root cause).

### 14. Arithmetic side effects — `rect.min + egui::vec2(...)`
`src/main.rs:187` — `rect.min + egui::vec2(item.x, item.y)`. `pedantic`'s `arithmetic_side_effects` flags *any* `+`/`-`/etc., including this one on `egui::Pos2 + egui::Vec2` (float vector addition via operator overload — can't overflow or panic). Reads as a false positive for this call site rather than a real bug; if kept enabled, scope an `#[allow(clippy::arithmetic_side_effects)]` on the block rather than restructuring working code around a lint that doesn't apply here.
Category: not a rust-idioms finding — noting it so it isn't mistaken for one.

### 15. Possible truncation — `file.metadata()?.len() as usize`
`src/main.rs:444` — `u64 as usize` truncates silently on 32-bit targets. Idiomatic fix per clippy's own suggestion: `usize::try_from(file.metadata()?.len()).map_err(std::io::Error::other)?` — surfaces the (practically impossible on a dev machine, but real on 32-bit) failure instead of wrapping the value.
Category: *type-driven-design* (`TryFrom` over an unchecked cast) + *errors* (`?` over silent data loss).

### 16. Unused `self` — `parse_response_headers`
`src/main.rs:513` — `fn parse_response_headers(&self, reader: &mut BufReader<NetworkStream>)` never reads `self` in its body (it only reads status line + headers off `reader`). Clippy's own suggestion applies: make it an associated function (drop `&self`, call site `:456` becomes `Self::parse_response_headers(&mut stream)`), which also loosens the signature — nothing about parsing response headers actually needs a `Url` receiver.
Category: *api-design* ("flexible" — signature shouldn't require what the body doesn't use).

### 17. Needless pass-by-value — `lex`'s `encoding: BodyEncoding`
`src/main.rs:563`, paired with clippy's own note at `:368` (the `enum BodyEncoding` declaration). `BodyEncoding` is `ContentLength(usize) | Chunked` — a tag plus one `usize`, trivially cheap to copy. Deriving `Copy` (`#[derive(Debug, Clone, Copy)]`, also picking up the missing `Debug` per API-design's "implement Debug nearly always") resolves the lint and matches the type's actual size — no reference plumbing needed for something this small.
Category: *api-design* (eager `Debug`/`Clone`/`Copy` for a small POD-shaped enum) + *ownership-and-performance* (Copy over indirection for tiny types).
