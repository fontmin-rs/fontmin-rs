pub use async_trait::async_trait;
use fontmin_core::Asset;
use fontmin_diagnostics::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginOrder {
    Pre,
    Normal,
    Post,
}

#[async_trait]
pub trait FontminPlugin: Send + Sync {
    fn name(&self) -> &'static str;

    fn order(&self) -> PluginOrder {
        PluginOrder::Normal
    }

    async fn build_start(&self) -> Result<()> {
        Ok(())
    }

    async fn transform(&self, asset: Asset) -> Result<Vec<Asset>> {
        Ok(vec![asset])
    }

    async fn generate_bundle(&self, _assets: &mut Vec<Asset>) -> Result<()> {
        Ok(())
    }

    async fn build_end(&self) -> Result<()> {
        Ok(())
    }
}
