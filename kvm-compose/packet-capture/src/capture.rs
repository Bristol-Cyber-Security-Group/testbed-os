use futures::StreamExt;
use pcap::{Capture, PacketCodec};
use crate::interface::OVSConfig;
use crate::TCPDumpConfig;

struct RawPacket;

impl PacketCodec for RawPacket {
    type Item = String;

    fn decode(&mut self, packet: pcap::Packet<'_>) -> Self::Item {
        format!("{packet:?}")
    }
}

pub async fn packet_capture(
    config: TCPDumpConfig,
    stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> anyhow::Result<()> {

    // create the OVS mirroring
    let mirror_port = OVSConfig::setup(&config).await?;

    // open connection to interface
    let capture = Capture::from_device(mirror_port.as_str())?
        .immediate_mode(true)
        .open()?
        // set non-blocking so we can check if we need to exit due to a stop instruction, otherwise
        // the `next_packet` function will block until the next packet before we are allows to check
        .setnonblock()?;

    // open a packet stream that can be used in futures
    let stream = capture.stream(RawPacket {})?;

    // this creates a stream, which is an async iterator, the stream creates a future for each
    // packet that is captured which is then processed inside the closure below
    let fut = stream.for_each(move |s| {
        tracing::info!("inside: {s:?}");

        // we need to return an empty future here, but this should be changed to a future that can
        // be processed into the destination data sink, keep this future light
        futures::future::ready(())
    });

    // wait on either the stop instruction being received, or the stream closing itself, so
    // whichever happens this will idiomatically and safely stop the processing of the packets
    tokio::select! {
        _ = stop_rx => {
            // we received a stop instruction
            tracing::info!("received stop instruction");
        }
        result = fut => {
            // the stream has closed itself, for some reason
            tracing::error!("packet capture stream stopped: {:?}", result);
        }
    }

    // destroy the mirror port and dummy interface
    OVSConfig::teardown(&config).await?;

    tracing::info!("capture complete");

    Ok(())
}