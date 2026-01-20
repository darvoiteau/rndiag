use std::net::{Ipv4Addr, Ipv6Addr, IpAddr};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crossterm::event::KeyModifiers;
use rndiag_graph::graph::graph_display;
use crossterm::event::{self, Event, KeyCode};

use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{MutableIpv4Packet, checksum as ipv4_checksum};
use pnet::packet::ipv6::MutableIpv6Packet;
use pnet::packet::tcp::MutableTcpPacket;
use pnet::packet::{Packet};
use pnet::transport::{
    transport_channel, TransportChannelType, ipv4_packet_iter, tcp_packet_iter,
};

use crate::tool::LatencyTool;

pub struct TCPPingTool {
    pub target: String,
    port: u16,
    data: Vec<u16>,
    begin_time: u64,
    elapsed_time: u64,
    sys_time: Vec<u64>,
    latency_time: Vec<u64>,
    latency_min: Vec<u16>,
    latency_moy: Vec<u16>,
    latency_max: Vec<u16>,
    latency_min_sampled: Vec<u64>,
    latency_moy_sampled: Vec<u64>,
    latency_max_sampled: Vec<u64>,
    output: String,
    nb_ping: u16,
    flag: u8,
}

impl LatencyTool for TCPPingTool {
    fn name(&self) -> &'static str {
        "tping"
    }

    fn data(&self) -> &Vec<u16> {
        &self.data
    }

    fn nb_ping(&self) -> &u16 {
        &self.nb_ping
    }

    fn sys_time(&self) -> &Vec<u64> {
        &self.sys_time
    }

    fn elapsed_time(&mut self) -> &mut u64 {
        &mut self.elapsed_time
    }

    fn begin_time(&self) -> &u64 {
        &self.begin_time
    }

    fn latency_time(&mut self) -> &mut Vec<u64> {
        &mut self.latency_time
    }

    fn latency_min(&mut self) -> &mut Vec<u16> {
        &mut self.latency_min
    }

    fn latency_moy(&mut self) -> &mut Vec<u16> {
        &mut self.latency_moy
    }

    fn latency_max(&mut self) -> &mut Vec<u16> {
        &mut self.latency_max
    }

    fn latency_min_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_min_sampled
    }

    fn latency_moy_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_moy_sampled
    }

    fn latency_max_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_max_sampled
    }

    fn output(&self) -> &str {
        &self.output
    }

    fn target(&self) -> &str {
        &self.target
    }

    async fn run(&mut self) -> std::io::Result<()> {
        self.tcp_ping(self.flag, 500).await?;
        Ok(())
    }
}

impl TCPPingTool {
    #[allow(unused_assignments)]
    async fn tcp_ping(&mut self, flags: u8, interval_ms: u64) -> std::io::Result<()> {
        let mut i: u16 = 0;
        let mut j: u16 = 0;

        // Resolve hostname or parse IP
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        
        if self.target.parse::<IpAddr>().is_ok() {
            target_ip = self.target.parse().expect("Invalid IP address");
        } else {
            target_ip = self.resolve();
        }

        // Bind to appropriate address family
        let bind_addr = match target_ip {
            IpAddr::V4(_) => "0.0.0.0:0",
            IpAddr::V6(_) => "[::]:0",
        };

        let socket = std::net::UdpSocket::bind(bind_addr)?;
        socket.connect(format!("{}:80", target_ip))?;
        let src_ip = socket.local_addr()?.ip();
        
        let src_port: u16 = 54321;

        // Use Layer3 for full control
        let protocol = TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp);

        let (mut sender, mut receiver) = transport_channel(4096, protocol)
            .expect("failed to open raw socket (need root/admin privileges)");
        
        let flags_str = decode_tcp_flags(flags);
        println!(
            "TCP-PING {}:{} from {}:{} flags=0x{:02x} ({}) count={}",
            target_ip, self.port, src_ip, src_port, flags, flags_str, self.nb_ping
        );

        let mut k: usize = 0;
        let mut scale: u16 = 5;
        let opt_graph = true;

        self.begin_time = self.get_time();

        // Main loop
        while i < self.nb_ping || self.nb_ping == 0 {
            // Enable raw terminal
            use crossterm::terminal::enable_raw_mode;
            enable_raw_mode()?;
            if event::poll(Duration::from_millis(20))? {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('g'), KeyModifiers::NONE) => {
                            k = self.sampling(k, scale);
                            graph_display(
                                &self.latency_min_sampled,
                                &self.latency_moy_sampled,
                                &self.latency_max_sampled,
                            ).unwrap_or_else(|e| {
                                eprintln!("Error during graph building: {}", e);
                            });
                        }
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            use crossterm::terminal::disable_raw_mode;
                            disable_raw_mode()?;
                            self.latency_data();
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }

            let start = Instant::now();

            // Build and send packet based on IP version
            let reply = match (src_ip, target_ip) {
                (IpAddr::V4(src), IpAddr::V4(dst)) => {
                    let mut buffer = [0u8; 40]; // IPv4: 20 + 20
                    build_ipv4_packet(&mut buffer, src, dst, src_port, self.port, flags);
                    
                    sender.send_to(
                        pnet::packet::ipv4::Ipv4Packet::new(&buffer).unwrap(),
                        std::net::IpAddr::V4(dst)
                    ).expect("send_to failed");
                    
                    wait_reply_ipv4(&mut receiver, dst, self.port, start).await
                }
                (IpAddr::V6(src), IpAddr::V6(dst)) => {
                    let mut buffer = [0u8; 60]; // IPv6: 40 + 20
                    build_ipv6_packet(&mut buffer, src, dst, src_port, self.port, flags);
                    
                    sender.send_to(
                        pnet::packet::ipv6::Ipv6Packet::new(&buffer).unwrap(),
                        std::net::IpAddr::V6(dst)
                    ).expect("send_to failed");
                    
                    wait_reply_ipv6(&mut receiver, dst, self.port, start).await
                }
                _ => {
                    eprintln!("IP version mismatch");
                    return Ok(());
                }
            };

            match reply {
                Some((latency, reply_flags)) => {
                    let flags_str = decode_tcp_flags(reply_flags);
                    use crossterm::terminal::disable_raw_mode;
                    disable_raw_mode()?;
                    println!(
                        "[{}] Reply in {:.3} ms - flags=0x{:02x} ({})",
                        i + 1,
                        latency.as_secs_f64() * 1000.0,
                        reply_flags,
                        flags_str
                    );
                    enable_raw_mode()?;

                    if latency.as_millis() as u16 >= 5000 {
                        self.data.push(5000);
                        self.sys_time.push(self.get_time());
                    } else {
                        self.data.push(latency.as_millis() as u16);
                        self.sys_time.push(self.get_time());
                    }

                    // Sampling logic
                    if j == scale && opt_graph {
                        j = 0;
                        let scale_changed: u16 = scale;

                        if self.elapsed_time <= 300 {
                            scale = 5;
                        } else if self.elapsed_time <= 1800 {
                            scale = 15;
                        } else if self.elapsed_time <= 3600 {
                            scale = 30;
                        } else if self.elapsed_time <= 7200 {
                            scale = 60;
                        } else if self.elapsed_time <= 14400 {
                            scale = 120;
                        } else if self.elapsed_time <= 28800 {
                            scale = 240;
                        } else if self.elapsed_time <= 57600 {
                            scale = 480;
                        } else if self.elapsed_time <= 115200 {
                            scale = 960;
                        } else if self.elapsed_time <= 230400 {
                            scale = 1920;
                        } else if self.elapsed_time <= 460800 {
                            scale = 3840;
                        } else if self.elapsed_time <= 921600 {
                            scale = 7680;
                        } else if self.elapsed_time <= 1843200 {
                            scale = 15360;
                        }

                        if scale_changed != scale {
                            k = 0;
                            self.latency_max_sampled.clear();
                            self.latency_moy_sampled.clear();
                            self.latency_min_sampled.clear();
                            k = self.sampling(k, 0);
                        }
                        k = self.sampling(k, scale);
                    } else if opt_graph {
                        j += 1;
                    }
                    i += 1;
                }
                None => {
                    panic!("Inconsistent data in packet response");
                }
            }

            // Wait interval
            if self.nb_ping != 0 {
                if interval_ms > 0 && i < self.nb_ping - 1 {
                    sleep(Duration::from_millis(interval_ms)).await;
                }
            } else {
                if interval_ms > 0 {
                    sleep(Duration::from_millis(interval_ms)).await;
                }
            }
        }

        use crossterm::terminal::disable_raw_mode;
        disable_raw_mode()?;
        self.latency_data();
        Ok(())
    }
}

// ========== IPv4 Functions ==========

fn build_ipv4_packet(
    buffer: &mut [u8],
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    flags: u8,
) {
    let mut ip_packet = MutableIpv4Packet::new(buffer).unwrap();
    ip_packet.set_version(4);
    ip_packet.set_header_length(5);
    ip_packet.set_total_length(40);
    ip_packet.set_ttl(64);
    ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ip_packet.set_source(src_ip);
    ip_packet.set_destination(dst_ip);
    ip_packet.set_checksum(0);
    
    let checksum = ipv4_checksum(&ip_packet.to_immutable());
    ip_packet.set_checksum(checksum);

    {
        let mut tcp = MutableTcpPacket::new(&mut buffer[20..40]).unwrap();
        tcp.set_source(src_port);
        tcp.set_destination(dst_port);
        tcp.set_sequence(12345);
        tcp.set_acknowledgement(0);
        tcp.set_data_offset(5);
        tcp.set_flags(flags);
        tcp.set_window(64240);
        tcp.set_urgent_ptr(0);
        tcp.set_checksum(0);
    }
    
    let cksum = tcp_checksum_ipv4(src_ip, dst_ip, &buffer[20..40]);
    let mut tcp = MutableTcpPacket::new(&mut buffer[20..40]).unwrap();
    tcp.set_checksum(cksum);
}

fn tcp_checksum_ipv4(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, tcp_packet: &[u8]) -> u16 {
    let mut sum = 0u32;

    for byte in src_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    for byte in dst_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    sum += IpNextHeaderProtocols::Tcp.0 as u32;
    sum += tcp_packet.len() as u32;

    let mut chunks = tcp_packet.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&b) = chunks.remainder().first() {
        sum += (b as u32) << 8;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

async fn wait_reply_ipv4(
    receiver: &mut pnet::transport::TransportReceiver,
    target_ip: Ipv4Addr,
    target_port: u16,
    start: Instant,
) -> Option<(Duration, u8)> {
    let timeout = Duration::from_millis(5000);

    loop {
        if start.elapsed() > timeout {
            return Some((timeout, 0));
        }

        let mut iter = ipv4_packet_iter(receiver);
        
        match iter.next() {
            Ok((packet, addr)) => {
                if let std::net::IpAddr::V4(src_ip) = addr {
                    if src_ip != target_ip {
                        continue;
                    }
                } else {
                    continue;
                }

                if packet.get_next_level_protocol() == IpNextHeaderProtocols::Tcp {
                    if let Some(tcp) = pnet::packet::tcp::TcpPacket::new(packet.payload()) {
                        if tcp.get_source() == target_port {
                            return Some((start.elapsed(), tcp.get_flags()));
                        }
                    }
                }
            }
            Err(_) => {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
        }
    }
}

// ========== IPv6 Functions ==========

fn build_ipv6_packet(
    buffer: &mut [u8],
    src_ip: Ipv6Addr,
    dst_ip: Ipv6Addr,
    src_port: u16,
    dst_port: u16,
    flags: u8,
) {
    let mut ip_packet = MutableIpv6Packet::new(buffer).unwrap();
    ip_packet.set_version(6);
    ip_packet.set_traffic_class(0);
    ip_packet.set_flow_label(0);
    ip_packet.set_payload_length(20);
    ip_packet.set_next_header(IpNextHeaderProtocols::Tcp);
    ip_packet.set_hop_limit(64);
    ip_packet.set_source(src_ip);
    ip_packet.set_destination(dst_ip);

    {
        let mut tcp = MutableTcpPacket::new(&mut buffer[40..60]).unwrap();
        tcp.set_source(src_port);
        tcp.set_destination(dst_port);
        tcp.set_sequence(12345);
        tcp.set_acknowledgement(0);
        tcp.set_data_offset(5);
        tcp.set_flags(flags);
        tcp.set_window(64240);
        tcp.set_urgent_ptr(0);
        tcp.set_checksum(0);
    }
    
    let cksum = tcp_checksum_ipv6(src_ip, dst_ip, &buffer[40..60]);
    let mut tcp = MutableTcpPacket::new(&mut buffer[40..60]).unwrap();
    tcp.set_checksum(cksum);
}

fn tcp_checksum_ipv6(src_ip: Ipv6Addr, dst_ip: Ipv6Addr, tcp_packet: &[u8]) -> u16 {
    let mut sum = 0u32;

    for byte in src_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    for byte in dst_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    
    let tcp_len = tcp_packet.len() as u32;
    sum += (tcp_len >> 16) as u32;
    sum += (tcp_len & 0xFFFF) as u32;
    sum += IpNextHeaderProtocols::Tcp.0 as u32;

    let mut chunks = tcp_packet.chunks_exact(2);
    for chunk in &mut chunks {
        sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
    }
    if let Some(&b) = chunks.remainder().first() {
        sum += (b as u32) << 8;
    }

    while (sum >> 16) != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !(sum as u16)
}

async fn wait_reply_ipv6(
    receiver: &mut pnet::transport::TransportReceiver,
    target_ip: Ipv6Addr,
    target_port: u16,
    start: Instant,
) -> Option<(Duration, u8)> {
    let timeout = Duration::from_millis(5000);

    loop {
        if start.elapsed() > timeout {
            return Some((timeout, 0));
        }

        // Use tcp_packet_iter which works for both IPv4 and IPv6
        let mut iter = tcp_packet_iter(receiver);
        
        match iter.next() {
            Ok((tcp, addr)) => {
                // Check if it's from our target
                if let std::net::IpAddr::V6(src_ip) = addr {
                    if src_ip != target_ip {
                        continue;
                    }
                } else {
                    continue;
                }

                if tcp.get_source() == target_port {
                    return Some((start.elapsed(), tcp.get_flags()));
                }
            }
            Err(_) => {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
        }
    }
}

// ========== Common Functions ==========

fn decode_tcp_flags(flags: u8) -> String {
    let mut result = Vec::new();
    
    if flags & 0x01 != 0 { result.push("FIN"); }
    if flags & 0x02 != 0 { result.push("SYN"); }
    if flags & 0x04 != 0 { result.push("RST"); }
    if flags & 0x08 != 0 { result.push("PSH"); }
    if flags & 0x10 != 0 { result.push("ACK"); }
    if flags & 0x20 != 0 { result.push("URG"); }
    
    if result.is_empty() {
        "NONE".to_string()
    } else {
        result.join("|")
    }
}

impl TCPPingTool {
    #[allow(dead_code)]
    fn setting(&mut self, output: &str, nb_ping: u16) {
        self.output = output.to_string();
        self.nb_ping = nb_ping;
    }

    pub fn new(target: &str, output: &str, nb_ping: u16, port: u16, flag: u8) -> Self {
        Self {
            target: target.to_string(),
            output: output.to_string(),
            port: port,
            nb_ping,
            data: Vec::new(),
            sys_time: Vec::new(),
            begin_time: 0,
            elapsed_time: 0,
            latency_time: Vec::new(),
            latency_min: Vec::new(),
            latency_moy: Vec::new(),
            latency_max: Vec::new(),
            latency_min_sampled: Vec::new(),
            latency_moy_sampled: Vec::new(),
            latency_max_sampled: Vec::new(),
            flag: flag,
        }
    }
}