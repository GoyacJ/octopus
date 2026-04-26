use async_trait::async_trait;
use harness_contracts::{ModelError, UsageSnapshot};
use http::HeaderMap;

use crate::{InferContext, ModelRequest, ModelStream};

#[async_trait]
pub trait InferMiddleware: Send + Sync + 'static {
    fn middleware_id(&self) -> &str;

    async fn before_request(
        &self,
        _req: &mut ModelRequest,
        _ctx: &mut InferContext,
    ) -> Result<(), ModelError> {
        Ok(())
    }

    async fn on_response_headers(
        &self,
        _headers: &HeaderMap,
        _ctx: &InferContext,
    ) -> Result<(), ModelError> {
        Ok(())
    }

    fn wrap_stream(&self, stream: ModelStream, _ctx: &InferContext) -> ModelStream {
        stream
    }

    async fn on_request_end(
        &self,
        _usage: &UsageSnapshot,
        _ctx: &InferContext,
    ) -> Result<(), ModelError> {
        Ok(())
    }
}
