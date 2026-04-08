use futures::{TryStreamExt};
use pcap::{Active, Capture, Packet, PacketCodec, PacketHeader};
use crate::interface::OVSConfig;
use crate::{TCPDumpConfig, TCPDumpConsumer};
use std::path::Path;
use anyhow::{bail, Context};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

#[cfg(feature = "cli-binary")]
use etherparse::{NetSlice::*, SlicedPacket};
#[cfg(feature = "cli-binary")]
use std::fmt::Write;

/// Temporary wrapper for packets captured by tcpdump to be used by consumers ``TCPDumpConsumer``.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketOwned {
    pub header: PacketHeader,
    pub data: Box<[u8]>,
}

impl PacketOwned {
    fn to_packet(&self) -> Packet<'_> {
        Packet::new(&self.header, &self.data)
    }
}

/// Implement a custom codec for use with ``PacketCodec`` to define how we want to parse all the
/// packets coming from tcpdump. We simply pass it on as-is using our basic ``PacketOwned``
/// implementation.
struct Codec;

impl PacketCodec for Codec {
    type Item = PacketOwned;

    fn decode(&mut self, packet: pcap::Packet<'_>) -> Self::Item {
        // print human-readable logs for debugging when using as CLI
        #[cfg(feature = "cli-binary")] {
            let print = decode_packet_human_readable(&packet);
            tracing::info!(print);
        }

        // we need to place Packet contents into PacketOwned as the pcap crate forces a restrictive
        // lifetime on Packet
        PacketOwned {
            header: *packet.header,
            data: packet.data.into(),
        }
    }
}

pub async fn packet_capture(
    config: &TCPDumpConfig,
    stop_rx: tokio::sync::oneshot::Receiver<()>,
    ovs_db_socket: String,
) -> anyhow::Result<()> {

    // create the OVS mirroring
    OVSConfig::setup(&config, ovs_db_socket.clone()).await
        .context("Setting up mirror port infrastructure")?;

    // open connection to interface
    let mut capture = Capture::from_device(config.mirror_interface.as_str())?
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

    // set up consumer and channel for sending packets from tcpdump try_for_each future to consumer
    let (tx, rx) = tokio::sync::mpsc::channel::<PacketOwned>(100);
    let consumer_handle = setup_consumer(&config.consumer, &capture, rx)
        .context("Setting up consumer")?;

    // open a packet stream that can be used in futures
    let stream = capture.stream(Codec {})?;

    // this creates a stream, which is an async iterator, the stream creates a future for each
    // packet that is captured which is then processed inside the closure below
    let fut = stream.try_for_each(async |s| {
        // send the captured packet through consumer channel
        let send_res = tx.send(s)
            .await;
        // need to manually handle any error in sending because we have to coerce the error into a
        // PcapError - this should be improved, not really supposed to do this .. leaving like this
        // just to move forward
        match send_res {
            Ok(_) => Ok(()), // returns Ok to continue loop
            Err(err) => {
                // exit this stream
                tracing::error!("Failed to send packet over channel to consumer");
                return Err(pcap::Error::PcapError(format!("Channel err: {err:#}")));
            }
        }
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
        consumer_res = consumer_handle => {
            // the consumer future exited for some reason
            tracing::error!("packet capture consumer stopped: {:?}", consumer_res);
        }
    }

    // destroy the mirror port and dummy interface
    OVSConfig::teardown(&config, ovs_db_socket).await
        .context("tearing down mirror port infrastructure")?;

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

/// The consumers created from ``TCPDumpConsumer`` will have their own logic in how they save the
/// packet data. All of these need to return a tokio future handle that will in the background
/// listen for packets so that they can be written to their consumer. This separates the concern of
/// tcpdump loop and the IO (consumer) loop.
fn setup_consumer(
    consumer: &TCPDumpConsumer,
    capture: &Capture<Active>,
    mut packet_rx: Receiver<PacketOwned>,
) -> anyhow::Result<JoinHandle<anyhow::Result<()>>> {
    match consumer {
        TCPDumpConsumer::File(path) => {
            // for file based, we create the pcap file first then supply a future that will be used
            // to read from the channel receiving `PacketOwned` to write into the pcap file.
            let pcap_path = Path::new(&path);
            if pcap_path.exists() {
                bail!("File already exists: {path:?}");
            }
            let mut savefile = capture.savefile(pcap_path)?;
            Ok(tokio::spawn(async move {
                loop {
                    let packet = packet_rx.recv().await;
                    match packet {
                        None => bail!("capture returned None"),
                        Some(packet_owned) => {
                            // need to reconstruct the packet
                            savefile.write(&packet_owned.to_packet())
                        }
                    }
                }
            }))
        }
    }
}
