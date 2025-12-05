mod interface;
mod capture;

pub struct TestbedPacketCapture;

impl TestbedPacketCapture {
    pub async fn capture(
        &self,
        interface: String, // TODO - interface configuration
        stop_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> anyhow::Result<()> {
        capture::packet_capture(interface, stop_rx).await?;
        Ok(())
    }
}
