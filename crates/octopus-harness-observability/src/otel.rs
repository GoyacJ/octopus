use crate::{AttributeValue, NoopTracer, Span, SpanAttributes, TraceCarrier, TraceContext, Tracer};

#[derive(Debug, Clone)]
pub struct OtelTracer {
    endpoint: String,
    service_name: String,
}

impl OtelTracer {
    /// Creates the M5 OpenTelemetry-compatible tracer shell.
    ///
    /// Exporter wiring is kept behind this type so the later exporter task can
    /// add batching without changing the public `Tracer` contract.
    pub fn new(
        endpoint: impl Into<String>,
        service_name: impl Into<String>,
    ) -> Result<Self, crate::ObservabilityError> {
        let endpoint = endpoint.into();
        let service_name = service_name.into();
        if endpoint.trim().is_empty() {
            return Err(crate::ObservabilityError::TracerInit(
                "otel endpoint is empty".to_owned(),
            ));
        }
        if service_name.trim().is_empty() {
            return Err(crate::ObservabilityError::TracerInit(
                "otel service name is empty".to_owned(),
            ));
        }
        Ok(Self {
            endpoint,
            service_name,
        })
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

impl Tracer for OtelTracer {
    fn start_span(&self, name: &str, attrs: SpanAttributes) -> Box<dyn Span> {
        let attrs = attrs
            .with(
                "otel.endpoint",
                AttributeValue::String(self.endpoint.clone()),
            )
            .with(
                "service.name",
                AttributeValue::String(self.service_name.clone()),
            );
        NoopTracer.start_span(name, attrs)
    }

    fn inject_context(&self, carrier: &mut dyn TraceCarrier) {
        NoopTracer.inject_context(carrier);
    }

    fn extract_context(&self, carrier: &dyn TraceCarrier) -> Option<TraceContext> {
        NoopTracer.extract_context(carrier)
    }
}
