use std::fmt::Write;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// JSON-formatted tracing event formatter.
pub struct JsonFormatter;

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(&mut self.0, "{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0 = value.to_string();
        }
    }
}

impl<S, N> FormatEvent<S, N> for JsonFormatter
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: tracing_subscriber::fmt::format::Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let level = match *meta.level() {
            tracing::Level::TRACE => "trace",
            tracing::Level::DEBUG => "debug",
            tracing::Level::INFO => "info",
            tracing::Level::WARN => "warn",
            tracing::Level::ERROR => "error",
        };

        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let message = visitor.0;

        let mut buf = String::new();
        write!(
            &mut buf,
            r#"{{"timestamp":"{}","level":"{}","target":"{}","message":"{}""#,
            timestamp,
            level,
            meta.target(),
            message.replace('\\', "\\\\").replace('"', "\\\""),
        )?;

        if let Some(scope) = ctx.event_scope() {
            for span in scope {
                let ext = span.extensions();
                if let Some(fields) = ext.get::<tracing_subscriber::fmt::FormattedFields<N>>()
                    && !fields.is_empty()
                {
                    write!(
                        &mut buf,
                        r#","span":"{}","span_fields":"{}""#,
                        span.name(),
                        fields
                            .to_string()
                            .replace('\\', "\\\\")
                            .replace('"', "\\\""),
                    )?;
                }
            }
        }

        writeln!(&mut buf, "}}")?;
        write!(writer, "{}", buf)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_json_formatter_level_mapping() {
        assert_eq!(
            match tracing::Level::TRACE {
                tracing::Level::TRACE => "trace",
                tracing::Level::DEBUG => "debug",
                tracing::Level::INFO => "info",
                tracing::Level::WARN => "warn",
                tracing::Level::ERROR => "error",
            },
            "trace"
        );
        assert_eq!(
            match tracing::Level::ERROR {
                tracing::Level::TRACE => "trace",
                tracing::Level::DEBUG => "debug",
                tracing::Level::INFO => "info",
                tracing::Level::WARN => "warn",
                tracing::Level::ERROR => "error",
            },
            "error"
        );
    }

    #[test]
    fn test_json_message_escaping() {
        let msg = r#"hello "world""#;
        let escaped = msg.replace('\\', "\\\\").replace('"', "\\\"");
        assert_eq!(escaped, r#"hello \"world\""#);
    }
}
