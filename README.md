# fastrace-tracing

[![Crates.io](https://img.shields.io/crates/v/fastrace-tracing.svg?style=flat-square&logo=rust)](https://crates.io/crates/fastrace-tracing)
[![Documentation](https://img.shields.io/docsrs/fastrace-tracing?style=flat-square&logo=rust)](https://docs.rs/fastrace-tracing/)
[![MSRV 1.80.0](https://img.shields.io/badge/MSRV-1.80.0-green?style=flat-square&logo=rust)](https://www.whatrustisit.com)
[![CI Status](https://img.shields.io/github/actions/workflow/status/fast/fastrace-tracing/ci.yml?style=flat-square&logo=github)](https://github.com/fast/fastrace-tracing/actions)
[![License](https://img.shields.io/crates/l/fastrace-tracing?style=flat-square)](https://github.com/fast/fastrace-tracing/blob/main/LICENSE)

A compatibility layer that connects [toiok-tracing](https://github.com/tokio-rs/tracing) with the [fastrace](https://github.com/fast/fastrace) tracing library.

## Overview

`fastrace-tracing` allows you to capture spans and events from libraries that use `tokio-tracing` and forward them to `fastrace`. This is particularly useful when:

- You're using `fastrace` in your application but depend on libraries instrumented with `tokio-tracing`
- You want to migrate from `tokio-tracing` to `fastrace` incrementally

## Getting Started

Add `fastrace-tracing` to your project:

```toml
[dependencies]
fastrace = { version = "0.7", features = ["enable"] }
fastrace-tracing = "0.1"
tracing = { version = "0.1", features = ["log"] }
tracing-subscriber = "0.3"
```

Set up the compatibility layer:

```rust
use fastrace::collector::{Config, ConsoleReporter};
use fastrace::prelude::*;
use tracing_subscriber::layer::SubscriberExt;

// Set up tokio-tracing with the fastrace compatibility layer.
let subscriber = tracing_subscriber::Registry::default()
    .with(fastrace_tracing::FastraceCompatLayer::new());
tracing::subscriber::set_global_default(subscriber).unwrap();

// Initialize fastrace.
fastrace::set_reporter(ConsoleReporter, Config::default());

// Initialize logging.
logforth::stderr().apply();

{
    // Create a fastrace root span.
    let root = Span::root("my-application", SpanContext::random());
    
    // Set a fastrace span as the local parent - this is critical for connecting the 
    // tokio-tracing spans with the fastrace span.
    let _guard = root.set_local_parent();

    // Spans from tokio-tracing will be captured by fastrace.
    let span = tracing::span!(tracing::Level::INFO, "my_operation");
    let _enter = span.enter();

    // Events from tokio-tracing will be captured by both fastrace and log.
    tracing::info!("This will be captured by fastrace");
}

// Flush any remaining traces before the program exits.
fastrace::flush();
```

## Examples

Check out the [examples directory](https://github.com/fast/fastrace-tracing/tree/main/examples) for more detailed usage examples.

## License

This project is licensed under the [Apache-2.0](./LICENSE) license.
