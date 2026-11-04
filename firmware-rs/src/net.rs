use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, IpEndpoint, Ipv4Address, Stack};

pub const STATIC_IP: Ipv4Address = Ipv4Address::new(10, 10, 10, 10);
pub const SUBNET_MASK: Ipv4Address = Ipv4Address::new(255, 255, 255, 0);

#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, embassy_usb::class::cdc_ncm::embassy_net::Device<'static, 1514>>) {
    runner.run().await;
}

#[embassy_executor::task]
pub async fn dhcp_server_task(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1024];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buf = [0u8; 1024];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buf,
        &mut tx_meta,
        &mut tx_buf,
    );

    if let Err(e) = socket.bind(67) {
        defmt::error!("DHCP bind port 67 failed: {:?}", e);
        return;
    }
    defmt::info!("DHCP server listening on UDP :67");

    let mut packet = [0u8; 576];

    loop {
        let (n, from) = match socket.recv_from(&mut packet).await {
            Ok(res) => res,
            Err(_) => continue,
        };

        if n < 240 {
            continue;
        }

        // Validate BOOTREQUEST (1) and magic cookie 99, 130, 83, 99
        if packet[0] != 1 || packet[236..240] != [99, 130, 83, 99] {
            continue;
        }

        let xid = [packet[4], packet[5], packet[6], packet[7]];
        let mut client_mac = [0u8; 6];
        client_mac.copy_from_slice(&packet[28..34]);

        // Parse options to determine message type
        let mut msg_type = 0u8;
        let mut idx = 240;
        while idx < n {
            let opt = packet[idx];
            if opt == 255 {
                break;
            }
            if opt == 0 {
                idx += 1;
                continue;
            }
            if idx + 1 >= n {
                break;
            }
            let len = packet[idx + 1] as usize;
            if opt == 53 && len == 1 && idx + 2 < n {
                msg_type = packet[idx + 2];
            }
            idx += 2 + len;
        }

        let assigned_ip = [10, 10, 10, 11];

        let reply_type = match msg_type {
            1 => 2, // DISCOVER -> OFFER
            3 => 5, // REQUEST -> ACK
            _ => continue,
        };

        // Construct DHCP reply packet
        let mut resp = [0u8; 300];
        resp[0] = 2; // BOOTREPLY
        resp[1] = 1; // Ethernet
        resp[2] = 6; // Hardware address length
        resp[3] = 0; // Hops
        resp[4..8].copy_from_slice(&xid);
        resp[8..10].copy_from_slice(&[0, 0]); // Secs
        resp[10..12].copy_from_slice(&[0x00, 0x00]); // Flags
        resp[12..16].copy_from_slice(&[0, 0, 0, 0]); // ciaddr
        resp[16..20].copy_from_slice(&assigned_ip); // yiaddr
        resp[20..24].copy_from_slice(&STATIC_IP.octets()); // siaddr
        resp[24..28].copy_from_slice(&[0, 0, 0, 0]); // giaddr
        resp[28..34].copy_from_slice(&client_mac);
        resp[236..240].copy_from_slice(&[99, 130, 83, 99]); // Magic cookie

        // Write DHCP options
        let mut opt_idx = 240;

        // Option 53: DHCP Message Type
        resp[opt_idx] = 53;
        resp[opt_idx + 1] = 1;
        resp[opt_idx + 2] = reply_type;
        opt_idx += 3;

        // Option 54: Server Identifier
        resp[opt_idx] = 54;
        resp[opt_idx + 1] = 4;
        resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&STATIC_IP.octets());
        opt_idx += 6;

        // Option 51: IP Address Lease Time (86400s = 1 day)
        resp[opt_idx] = 51;
        resp[opt_idx + 1] = 4;
        resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&[0x00, 0x01, 0x51, 0x80]);
        opt_idx += 6;

        // Option 1: Subnet Mask
        resp[opt_idx] = 1;
        resp[opt_idx + 1] = 4;
        resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&SUBNET_MASK.octets());
        opt_idx += 6;

        // Option 3: Router
        resp[opt_idx] = 3;
        resp[opt_idx + 1] = 4;
        resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&STATIC_IP.octets());
        opt_idx += 6;

        // Option 6: Domain Name Server
        resp[opt_idx] = 6;
        resp[opt_idx + 1] = 4;
        resp[opt_idx + 2..opt_idx + 6].copy_from_slice(&STATIC_IP.octets());
        opt_idx += 6;

        // Option 255: End
        resp[opt_idx] = 255;
        opt_idx += 1;

        let dest = IpEndpoint::new(IpAddress::v4(255, 255, 255, 255), 68);
        defmt::info!("DHCP sending {} to {:?}", if reply_type == 2 { "OFFER" } else { "ACK" }, from);
        let _ = socket.send_to(&resp[..opt_idx], dest).await;
    }
}

#[embassy_executor::task]
pub async fn dns_server_task(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 512];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buf = [0u8; 512];

    let mut socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buf,
        &mut tx_meta,
        &mut tx_buf,
    );

    if let Err(e) = socket.bind(53) {
        defmt::error!("DNS bind port 53 failed: {:?}", e);
        return;
    }
    defmt::info!("Captive DNS server listening on UDP :53");

    let mut query = [0u8; 256];

    loop {
        let (n, from) = match socket.recv_from(&mut query).await {
            Ok(res) => res,
            Err(_) => continue,
        };

        if n < 12 {
            continue;
        }

        // Check QR flag = 0 (Query)
        if (query[2] & 0x80) != 0 {
            continue;
        }

        // Build DNS response in-place
        let mut resp = [0u8; 280];
        resp[..n].copy_from_slice(&query[..n]);

        // Flags: QR=1 (response), AA=1, RA=1, RCODE=0 -> 0x8180
        resp[2] = 0x81;
        resp[3] = 0x80;

        // ANCOUNT = 1
        resp[6] = 0x00;
        resp[7] = 0x01;

        // NSCOUNT = 0, ARCOUNT = 0
        resp[8] = 0x00;
        resp[9] = 0x00;
        resp[10] = 0x00;
        resp[11] = 0x00;

        // Append Answer Record pointing to question name (0xC00C)
        let mut an_idx = n;
        resp[an_idx] = 0xC0; // Pointer tag
        resp[an_idx + 1] = 0x0C; // Offset 12
        an_idx += 2;

        // Type A (1)
        resp[an_idx] = 0x00;
        resp[an_idx + 1] = 0x01;
        an_idx += 2;

        // Class IN (1)
        resp[an_idx] = 0x00;
        resp[an_idx + 1] = 0x01;
        an_idx += 2;

        // TTL: 60 seconds
        resp[an_idx] = 0x00;
        resp[an_idx + 1] = 0x00;
        resp[an_idx + 2] = 0x00;
        resp[an_idx + 3] = 0x3C;
        an_idx += 4;

        // Data length: 4 bytes
        resp[an_idx] = 0x00;
        resp[an_idx + 1] = 0x04;
        an_idx += 2;

        // IP address: 10.10.10.10
        resp[an_idx..an_idx + 4].copy_from_slice(&STATIC_IP.octets());
        an_idx += 4;

        defmt::info!("DNS resolved query to 10.10.10.10 for {:?}", from);
        let _ = socket.send_to(&resp[..an_idx], from).await;
    }
}
