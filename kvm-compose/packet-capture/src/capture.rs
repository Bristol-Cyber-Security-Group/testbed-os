use futures::StreamExt;
use pcap::{Capture, PacketCodec};
use crate::interface::OVSConfig;
use crate::TCPDumpConfig;
#[cfg(feature = "cli-binary")]
use etherparse::{NetSlice::*, SlicedPacket};
#[cfg(feature = "cli-binary")]
use std::fmt::Write;

struct RawPacket;

impl PacketCodec for RawPacket {
    type Item = String;

    fn decode(&mut self, packet: pcap::Packet<'_>) -> Self::Item {
        // print human-readable logs for debugging when using as CLI
        #[cfg(feature = "cli-binary")] {
            let print = decode_packet_human_readable(&packet);
            tracing::info!(print);
        }
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
    let mut capture = Capture::from_device(mirror_port.as_str())?
        .immediate_mode(true)
        .open()?
        // set non-blocking so we can check if we need to exit due to a stop instruction, otherwise
        // the `next_packet` function will block until the next packet before we are allows to check
        .setnonblock()?;

    // if there are any filters given we need to apply
    if !config.dump_args.is_empty() {
        let filter = &config.dump_args.join(" ");
        tracing::info!("Applying BPF filter: {filter}");
        capture.filter(&filter, true)?;
    }

    // open a packet stream that can be used in futures
    let stream = capture.stream(RawPacket {})?;

    // this creates a stream, which is an async iterator, the stream creates a future for each
    // packet that is captured which is then processed inside the closure below
    let fut = stream.for_each(move |s| {
        // tracing::info!("inside: {s:?}");

        // TODO - push packet into consumer to either write to file or into a DB

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

/// This is just for debugging
#[cfg(feature = "cli-binary")]
fn decode_packet_human_readable(packet: &pcap::Packet) -> String {
    let mut output = String::new();

    let timestamp = packet.header.ts;
    let _ = write!(output, "{}.{:06} (len: {})", timestamp.tv_sec, timestamp.tv_usec, packet.header.len);

    match SlicedPacket::from_ethernet(packet.data) {
        Ok(sliced_packet) => {

            match sliced_packet.net {
                Some(Ipv4(ipv4)) => {
                    let _ = write!(output, " Ipv4 {:?} => {:?}", ipv4.header().source_addr(), ipv4.header().destination_addr());
                },
                Some(Ipv6(ipv6)) => {
                    let _ = write!(output, " Ipv6 {:?} => {:?}", ipv6.header().source_addr(), ipv6.header().destination_addr());
                }
                Some(Arp(arp)) => {
                    let _ = write!(output, " Arp {:?}", arp);
                }
                None => {}
            }

        }
        Err(e) => {
            let _ = write!(output, "{e}");
        }
    }
    output
}
