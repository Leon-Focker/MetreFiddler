# MetreFiddler

MetreFiddler is a MIDI event processing plugin that applies an advanced adaptation of Clarence Barlow’s ideas of [metric indispensability](https://leon-focker.github.io/metrical-hierarchies/). It assigns new velocity values to rhythmic events and filters them based on their metric weight. Metric structures are defined using [RQQ](https://michael-edwards.org/sc/manual/rhythms.html#rqq) notation.

This Plugin is built using the [nih_plug framework](https://github.com/robbert-vdh/nih-plug) with a GUI powered by vizia.

## Building

In the future, precompiled binaries can be found in the [Releases tab](https://github.com/Leon-Focker/MetreFiddler/releases/)

After installing [Rust](https://rustup.rs/), you can compile MetreFiddler yourself as follows:

```shell
cargo xtask bundle metrefiddler --release
```
