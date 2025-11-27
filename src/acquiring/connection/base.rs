use crate::acquiring::types::ConnectionConfig;

#[async_trait::async_trait]
pub trait BaseConnection: Send + Sync {
    fn config(&self) -> &ConnectionConfig;
    fn is_connected(&self) -> bool;
    async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>>;
    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn write(&mut self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
    async fn read(&mut self, timeout_ms: Option<u32>) -> Result<Vec<u8>, Box<dyn std::error::Error>>;
}

