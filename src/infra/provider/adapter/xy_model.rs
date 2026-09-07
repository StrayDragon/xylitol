//! The single-layer `XyModel` shell over a bridge adapter (pa1).

use async_trait::async_trait;

use crate::infra::provider::map::{to_bridge_tools, to_xy_error, to_xy_stream};
use crate::protocol::error::XyError;
use crate::protocol::message::LlmMessage;
use crate::protocol::model::XyToolSchema;
use crate::protocol::ports::{XyGenerateOptions, XyModel, XyStream};
use xylitol_ai_bridge::provider::AdapterRef as AiBridgeAdapterRef;

/// An [`XyModel`] backed directly by a bridge `AiBridgeLlmAdapter`; the
/// DTO→`XyChunk` mapping runs inline via the infra `map` module (pa7: vendor
/// types stop at this infra mapping boundary).
pub struct AdapterXyModel {
    adapter: AiBridgeAdapterRef,
}

impl AdapterXyModel {
    /// Wrap a bridge adapter as an `XyModel`.
    pub fn new(adapter: AiBridgeAdapterRef) -> Self {
        Self { adapter }
    }
}

#[async_trait]
impl XyModel for AdapterXyModel {
    fn name(&self) -> &str {
        self.adapter.name()
    }

    async fn generate_stream(
        &self,
        messages: Vec<LlmMessage>,
        tools: &[XyToolSchema],
        stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError> {
        let bridge_tools = to_bridge_tools(tools);
        if stream {
            let bridge_stream = self
                .adapter
                .generate_stream(messages, &bridge_tools, options)
                .await
                .map_err(to_xy_error)?;
            Ok(to_xy_stream(bridge_stream))
        } else {
            let bridge_stream = self
                .adapter
                .generate(messages, &bridge_tools, options)
                .await
                .map_err(to_xy_error)?;
            Ok(to_xy_stream(bridge_stream))
        }
    }
}
