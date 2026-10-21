use std::pin::Pin;

use async_trait::async_trait;

use crate::error::XyError;
use crate::message::XyLlmMessage;
use crate::model::XyChunk;
use crate::model::XyModelConfig;

pub type XyStream = Pin<Box<dyn futures::Stream<Item = Result<XyChunk, XyError>> + Send>>;

pub type XyModelBuilder =
    std::sync::Arc<dyn Fn(&XyModelConfig) -> std::sync::Arc<dyn XyModel> + Send + Sync>;

pub type XyGenerateOptions = crate::generate::XyGenerateOptions;

#[async_trait]
pub trait XyModel: Send + Sync {
    fn name(&self) -> &str;

    async fn generate_stream(
        &self,
        messages: Vec<XyLlmMessage>,
        tools: &[crate::model::XyToolSchema],
        stream: bool,
        options: XyGenerateOptions,
    ) -> Result<XyStream, XyError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::THINKING_OFF;

    #[test]
    fn xy_stream_type_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<XyStream>();
    }

    #[test]
    fn generate_options_default_is_off() {
        let opts = XyGenerateOptions::default();
        assert_eq!(opts.thinking_level, THINKING_OFF);
        assert!(opts.level_map.is_empty());
        assert!(opts.thinking_budgets.is_none());
    }
}
