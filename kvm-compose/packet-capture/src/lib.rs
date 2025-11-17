use pcap::Capture;

struct Packet {
    timestamp: u64,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
}

async fn packet_capture(

) -> anyhow::Result<()> {
    let mut capture = Capture::from_device("vm-ovn00")?
        .immediate_mode(true)
        .open()?;

    while let Ok(packet) = capture.next_packet() {
        println!("{:#?}", packet);
    }

    Ok(())
}
