#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::cell::LazyCell;
use std::fmt;
use std::marker;
use std::thread;

use fastrace::prelude::SpanContext;
use tracing_core::Event;
use tracing_core::Subscriber;
use tracing_core::field;
use tracing_core::span::Attributes;
use tracing_core::span::Id;
use tracing_core::span::Record;
use tracing_core::span::{self};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;

const FIELD_EXCEPTION_MESSAGE: &str = "exception.message";
const FIELD_EXCEPTION_STACKTRACE: &str = "exception.stacktrace";

/// A compatibility layer for using libraries instrumented with
/// `tokio-tracing` in applications using `fastrace`.
///
/// This layer collects spans and events created using the tokio-tracing ecosystem and
/// forwards them to fastrace's collectors.
///
/// # Example
///
/// ```
/// use fastrace::collector::Config;
/// use fastrace::collector::ConsoleReporter;
/// use fastrace::prelude::SpanContext;
/// use fastrace_tracing::FastraceCompatLayer;
/// use tracing_subscriber::layer::SubscriberExt;
///
/// // Set up fastrace reporter.
/// fastrace::set_reporter(ConsoleReporter, Config::default());
///
/// // Create a tracing subscriber with the FastraceCompatLayer.
/// let subscriber = tracing_subscriber::Registry::default().with(FastraceCompatLayer::new());
/// tracing::subscriber::set_global_default(subscriber).unwrap();
///
/// // Create a fastrace root span.
/// let root = fastrace::Span::root("root", SpanContext::random());
///
/// // Set a fastrace span as the local parent - this is critical for connecting the
/// // tokio-tracing spans with the fastrace span.
/// let _guard = root.set_local_parent();
///
/// // Spans from tokio-tracing will be captured by fastrace.
/// let span = tracing::span!(tracing::Level::TRACE, "my_span");
/// let _enter = span.enter();
///
/// // Events from tokio-tracing will also be captured by fastrace.
/// tracing::info!("This event will be captured by fastrace");
/// ```
pub struct FastraceCompatLayer<S> {
    location: bool,
    with_threads: bool,
    with_level: bool,
    _phantom: marker::PhantomData<S>,
}

struct EventNameFinder {
    name: Option<Cow<'static, str>>,
}

impl field::Visit for EventNameFinder {
    fn record_bool(&mut self, field: &field::Field, value: bool) {
        if field.name() == "message" {
            self.name = Some(value.to_string().into())
        }
    }

    fn record_f64(&mut self, field: &field::Field, value: f64) {
        if field.name() == "message" {
            self.name = Some(value.to_string().into())
        }
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        if field.name() == "message" {
            self.name = Some(value.to_string().into())
        }
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        if field.name() == "message" {
            self.name = Some(value.to_string().into())
        }
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.name = Some(format!("{:?}", value).into())
        }
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if field.name() == "message" {
            self.name = Some(value.to_string().into())
        }
    }
}

struct EventVisitor<'a> {
    fastrace_event: &'a mut fastrace::Event,
}

impl field::Visit for EventVisitor<'_> {
    fn record_bool(&mut self, field: &field::Field, value: bool) {
        if field.name() == "message" {
            return;
        }

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_f64(&mut self, field: &field::Field, value: f64) {
        if field.name() == "message" {
            return;
        }

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        if field.name() == "message" {
            return;
        }

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        if field.name() == "message" {
            return;
        }

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            return;
        }

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), format!("{:?}", value)))
        });
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if field.name() == "message" {
            return;
        }

        let mut chain: Vec<String> = Vec::new();
        let mut next_err = value.source();

        while let Some(err) = next_err {
            chain.push(err.to_string());
            next_err = err.source();
        }

        let error_msg = value.to_string();

        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (field.name(), error_msg.to_string()))
        });
        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (FIELD_EXCEPTION_MESSAGE, error_msg.to_string()))
        });
        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (format!("{}.chain", field.name()), format!("{:?}", chain)))
        });
        take_mut::take(self.fastrace_event, |event| {
            event.with_property(|| (FIELD_EXCEPTION_STACKTRACE, format!("{:?}", chain)))
        });
    }
}

struct SpanAttributeVisitor<'a> {
    fastrace_span: &'a mut fastrace::Span,
}

impl field::Visit for SpanAttributeVisitor<'_> {
    fn record_bool(&mut self, field: &field::Field, value: bool) {
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_f64(&mut self, field: &field::Field, value: f64) {
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_i64(&mut self, field: &field::Field, value: i64) {
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_str(&mut self, field: &field::Field, value: &str) {
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), value.to_string()))
        });
    }

    fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), format!("{:?}", value)))
        });
    }

    fn record_error(
        &mut self,
        field: &tracing_core::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        let mut chain: Vec<String> = Vec::new();
        let mut next_err = value.source();

        while let Some(err) = next_err {
            chain.push(err.to_string());
            next_err = err.source();
        }

        let error_msg = value.to_string();

        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (field.name(), error_msg.to_string()))
        });
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (FIELD_EXCEPTION_MESSAGE, error_msg.to_string()))
        });
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (format!("{}.chain", field.name()), format!("{:?}", chain)))
        });
        take_mut::take(self.fastrace_span, |span| {
            span.with_property(|| (FIELD_EXCEPTION_STACKTRACE, format!("{:?}", chain)))
        });
    }
}

impl<S> FastraceCompatLayer<S>
where S: Subscriber + for<'span> LookupSpan<'span>
{
    /// Creates a new [`FastraceCompatLayer`] with default settings.
    pub fn new() -> Self {
        FastraceCompatLayer {
            location: true,
            with_threads: true,
            with_level: false,
            _phantom: marker::PhantomData,
        }
    }

    /// Configures whether source code location information is included in spans.
    ///
    /// When enabled, span properties will include:
    /// - `code.filepath`: The file where the span was created
    /// - `code.namespace`: The module path where the span was created
    /// - `code.lineno`: The line number where the span was created
    ///
    /// Default is `true`.
    pub fn with_location(self, location: bool) -> Self {
        Self { location, ..self }
    }

    /// Configures whether thread information is included in spans.
    ///
    /// When enabled, span properties will include:
    /// - `thread.id`: The numeric ID of the thread
    /// - `thread.name`: The name of the thread (if available)
    ///
    /// Default is `true`.
    pub fn with_threads(self, threads: bool) -> Self {
        Self {
            with_threads: threads,
            ..self
        }
    }

    /// Configures whether level information is included in span properties.
    ///
    /// When enabled, spans will include the tracing level (trace, debug, info, etc.)
    /// as a property named `level`.
    ///
    /// Default is `false`.
    pub fn with_level(self, level: bool) -> Self {
        Self {
            with_level: level,
            ..self
        }
    }

    fn new_fastrace_span(&self, attrs: &Attributes<'_>, ctx: &Context<'_, S>) -> fastrace::Span {
        if let Some(parent) = attrs.parent() {
            // A span can have an _explicit_ parent that is NOT seen by this `Layer` (for which
            // `Context::span` returns `None`. This happens if the parent span is filtered away
            // from the layer by a per-layer filter. In that case, we fall-through to the `else`
            // case, and consider this span a root span.
            if let Some(span) = ctx.span(parent) {
                let extensions = span.extensions();
                return extensions
                    .get::<fastrace::Span>()
                    .map(|parent| {
                        fastrace::Span::enter_with_parent(attrs.metadata().name(), parent)
                    })
                    .unwrap_or_default();
            }
        }

        // Else if the span is inferred from context, look up any available current span.
        if attrs.is_contextual() {
            ctx.lookup_current()
                .and_then(|span| {
                    let extensions = span.extensions();
                    extensions.get::<fastrace::Span>().map(|parent| {
                        fastrace::Span::enter_with_parent(attrs.metadata().name(), parent)
                    })
                })
                .or_else(|| {
                    SpanContext::current_local_parent()
                        .map(|_| fastrace::Span::enter_with_local_parent(attrs.metadata().name()))
                })
                .unwrap_or_else(|| {
                    fastrace::Span::root(attrs.metadata().name(), SpanContext::random())
                })
        // Explicit root spans should have no parent context.
        } else {
            fastrace::Span::root(attrs.metadata().name(), SpanContext::random())
        }
    }
}

impl<S> Default for FastraceCompatLayer<S>
where S: Subscriber + for<'span> LookupSpan<'span>
{
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static THREAD_ID: LazyCell<u64> = LazyCell::new(|| {
        thread_id_integer(thread::current().id())
    });
}

fn thread_id_integer(id: thread::ThreadId) -> u64 {
    let thread_id = format!("{:?}", id);
    thread_id
        .trim_start_matches("ThreadId(")
        .trim_end_matches(')')
        .parse::<u64>()
        .expect("thread ID should parse as an integer")
}

impl<S> Layer<S> for FastraceCompatLayer<S>
where S: Subscriber + for<'span> LookupSpan<'span>
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");

        let mut fastrace_span = self.new_fastrace_span(attrs, &ctx);

        if self.location {
            let meta = attrs.metadata();

            if let Some(filename) = meta.file() {
                fastrace_span =
                    fastrace_span.with_property(|| ("code.filepath", filename.to_string()));
            }

            if let Some(module) = meta.module_path() {
                fastrace_span =
                    fastrace_span.with_property(|| ("code.namespace", module.to_string()));
            }

            if let Some(line) = meta.line() {
                fastrace_span = fastrace_span.with_property(|| ("code.lineno", line.to_string()));
            }
        }

        if self.with_threads {
            THREAD_ID.with(|id| {
                take_mut::take(&mut fastrace_span, |span| {
                    span.with_property(|| ("thread.id", id.to_string()))
                });
            });
            if let Some(name) = std::thread::current().name() {
                fastrace_span = fastrace_span.with_property(|| ("thread.name", name.to_string()));
            }
        }

        if self.with_level {
            fastrace_span =
                fastrace_span.with_property(|| ("level", attrs.metadata().level().as_str()));
        }

        attrs.record(&mut SpanAttributeVisitor {
            fastrace_span: &mut fastrace_span,
        });

        let mut extensions = span.extensions_mut();
        extensions.insert(fastrace_span);
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extension = span.extensions_mut();
        let Some(fastrace_span) = extension.get_mut::<fastrace::Span>() else {
            return;
        };
        values.record(&mut SpanAttributeVisitor { fastrace_span });
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Ignore events that are not in the context of a span
        if let Some(span) = event.parent().and_then(|id| ctx.span(id)).or_else(|| {
            event
                .is_contextual()
                .then(|| ctx.lookup_current())
                .flatten()
        }) {
            let mut extensions = span.extensions_mut();
            let fastrace_span = extensions.get_mut::<fastrace::Span>();

            if let Some(fastrace_span) = fastrace_span {
                let mut name_finder = EventNameFinder { name: None };
                event.record(&mut name_finder);
                let event_name = name_finder
                    .name
                    .unwrap_or_else(|| Cow::Borrowed(event.metadata().name()));

                let mut fastrace_event = fastrace::Event::new(event_name).with_properties(|| {
                    [
                        ("level", event.metadata().level().as_str().to_string()),
                        ("target", event.metadata().target().to_string()),
                    ]
                });

                if self.location {
                    if let Some(file) = event.metadata().file() {
                        fastrace_event =
                            fastrace_event.with_property(|| ("code.filepath", file.to_string()));
                    }
                    if let Some(module) = event.metadata().module_path() {
                        fastrace_event =
                            fastrace_event.with_property(|| ("code.namespace", module.to_string()));
                    }
                    if let Some(line) = event.metadata().line() {
                        fastrace_event =
                            fastrace_event.with_property(|| ("code.lineno", line.to_string()));
                    }
                }

                event.record(&mut EventVisitor {
                    fastrace_event: &mut fastrace_event,
                });

                fastrace_span.add_event(fastrace_event);
            }
        };
    }
}
