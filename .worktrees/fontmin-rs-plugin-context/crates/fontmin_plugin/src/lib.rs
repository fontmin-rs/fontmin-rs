pub use async_trait::async_trait;
use fontmin_core::Asset;
use fontmin_diagnostics::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginOrder {
    Pre,
    Normal,
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Loader,
    Transform,
    Generator,
    Reporter,
}

#[derive(Debug, Default)]
pub struct PluginContext {}

impl PluginContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
pub trait FontminPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    fn order(&self) -> PluginOrder {
        PluginOrder::Normal
    }

    fn kind(&self) -> PluginKind {
        PluginKind::Transform
    }

    async fn build_start(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }

    async fn transform(&self, _ctx: &mut PluginContext, asset: Asset) -> Result<Vec<Asset>> {
        Ok(vec![asset])
    }

    async fn generate_bundle(
        &self,
        _ctx: &mut PluginContext,
        _assets: &mut Vec<Asset>,
    ) -> Result<()> {
        Ok(())
    }

    async fn build_end(&self, _ctx: &mut PluginContext) -> Result<()> {
        Ok(())
    }
}
