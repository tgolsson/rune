# Objects

Objects are anonymous hash maps, which support defining arbitrary string keys.

```rust,noplaypen
{{#include ../../scripts/book/5_3/objects.rn}}
```

```text
$> cargo run -- scripts/book/5_3/objects.rn
"bar"
42
== () (7.9265ms)
```

These are useful because they allow their data to be specified dynamically,
which is exactly the same use case as storing unknown JSON.

One of the largest motivations for *Rune* to have anonymous objects is so that
we can handle JSON with an unknown structure.

```rust,noplaypen
{{#include ../../scripts/book/5_3/json.rn}}
```

```text
$> cargo run -- scripts/book/5_3/json.rn
9c4bdaf194410d8b2f5d7f9f52eb3e64709d3414
06419f2580e7a18838f483321055fc06c0d75c4c
cba225dad143779a0a9543cfb05cde9710083af5
15133745237c014ff8bae53d8ff8f3c137c732c7
39ac97ab4ebe26118e807eb91c7656ab95b1fcac
3f6310eeeaca22d0373cc11d8b34d346bd12a364
== () (331.3324ms)
```