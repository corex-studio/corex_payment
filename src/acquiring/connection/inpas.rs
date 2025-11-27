use crate::acquiring::connection::BaseConnection;
use crate::acquiring::types::ConnectionConfig;

pub struct InpasConnection {
  config: ConnectionConfig,
  connected: bool,
}

impl InpasConnection {
  pub fn new(config: ConnectionConfig) -> Self {
    Self {
      config,
      connected: false,
    }
  }
}

#[async_trait::async_trait]
impl BaseConnection for InpasConnection {
  fn config(&self) -> &ConnectionConfig {
    &self.config
  }

  fn is_connected(&self) -> bool {
    self.connected
  }

  async fn connect(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
    self.connected = true;
    Ok(true)
  }

  async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    self.connected = false;
    Ok(())
  }

  async fn write(&mut self, _data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    Err("write is not supported for inpas protocol".into())
  }

  async fn read(
    &mut self,
    _timeout_ms: Option<u32>,
  ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    Err("read is not supported for inpas protocol".into())
  }
}
