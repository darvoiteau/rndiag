use std::net::{Ipv4Addr, Ipv6Addr, IpAddr};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crossterm::event::KeyModifiers;
use rndiag_graph::graph::graph_display;
use crossterm::event::{self, Event, KeyCode};

use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{MutableIpv4Packet, checksum as ipv4_checksum};
use pnet::packet::tcp::MutableTcpPacket;
use pnet::packet::Packet;
use pnet::transport::{
    transport_channel, TransportChannelType, ipv4_packet_iter,
};
use socket2::{Socket, Domain, Type, Protocol};

use crate::tool::LatencyTool;

// ─────────────────────────────────────────────────────────────────────────────
// TCPPingTool struct definition
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// LatencyTool trait implementation
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// TCPPingTool-specific methods
// ─────────────────────────────────────────────────────────────────────────────

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

        // Bind to appropriate address family to discover the local source IP
        let bind_addr = match target_ip {
            IpAddr::V4(_) => "0.0.0.0:0",
            IpAddr::V6(_) => "[::]:0",
        };

        let socket = std::net::UdpSocket::bind(bind_addr)?;
        socket.connect(format!("{}:80", target_ip))?;
        let src_ip = socket.local_addr()?.ip();

        let src_port: u16 = 54321;

        // IPv4 only: open a Layer3 raw socket for full IP+TCP control
        // IPv6: uses socket2 raw socket — handles arbitrary TCP flags correctly
        let mut ipv4_channel = match target_ip {
            IpAddr::V4(_) => {
                let protocol = TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp);
                Some(
                    transport_channel(4096, protocol)
                        .expect("failed to open IPv4 raw socket (need root/admin privileges)"),
                )
            }
            IpAddr::V6(_) => None,
        };

        let flags_str = decode_tcp_flags(flags);
        println!(
            "TCP-PING {}:{} from {}:{} flags=0x{:02x} ({}) count={}",
            target_ip, self.port, src_ip, src_port, flags, flags_str, self.nb_ping
        );

        let mut k: usize = 0;
        let mut scale: u16 = 5;
        let opt_graph = true;

        self.begin_time = self.get_time();

        // Main loop: runs until nb_ping is reached, or forever if nb_ping == 0
        while i < self.nb_ping || self.nb_ping == 0 {
            use crossterm::terminal::{enable_raw_mode, disable_raw_mode};

            enable_raw_mode()?;

            if event::poll(Duration::from_millis(20))? {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match (key_event.code, key_event.modifiers) {
                        // 'g' => compute latest sampling then show the graph (blocking)
                        (KeyCode::Char('g'), KeyModifiers::NONE) => {
                            k = self.sampling(k, scale);
                            graph_display(
                                &self.latency_min_sampled,
                                &self.latency_moy_sampled,
                                &self.latency_max_sampled,
                            )
                            .unwrap_or_else(|e| {
                                eprintln!("Error during graph building: {}", e);
                            });
                        }
                        // Ctrl-C => print stats then exit cleanly
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            disable_raw_mode()?;
                            self.latency_data();
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }

            let start = Instant::now();

            // Dispatch per IP version:
            // IPv4 => pnet Layer3 raw socket, full IP+TCP control, real TCP flags in reply
            // IPv6 => socket2 raw socket, kernel adds IPv6 header, arbitrary TCP flags supported
            let reply: Option<(Duration, u8)> = match target_ip {
                IpAddr::V4(dst) => {
                    let src = match src_ip {
                        IpAddr::V4(a) => a,
                        _ => unreachable!(),
                    };
                    let (sender, receiver) = ipv4_channel.as_mut().unwrap();
                    let mut buffer = [0u8; 40]; // IPv4 (20) + TCP (20)
                    build_ipv4_packet(&mut buffer, src, dst, src_port, self.port, flags);

                    sender
                        .send_to(
                            pnet::packet::ipv4::Ipv4Packet::new(&buffer).unwrap(),
                            std::net::IpAddr::V4(dst),
                        )
                        .expect("send_to failed");

                    wait_reply_ipv4(receiver, dst, self.port, start).await
                }
                IpAddr::V6(dst) => {
                    let src = match src_ip {
                        IpAddr::V6(a) => a,
                        _ => unreachable!(),
                    };

                    // Raw IPv6 socket: kernel adds the IPv6 header, we only provide TCP segment
                    let send_sock = Socket::new(
                        Domain::IPV6,
                        Type::RAW,
                        Some(Protocol::from(6)), // IPPROTO_TCP = 6
                    )
                    .expect("failed to create IPv6 raw send socket (need root)");

                    let mut tcp_buffer = [0u8; 20];
                    build_tcp_packet_v6(&mut tcp_buffer, src, dst, src_port, self.port, flags);

                    let dst_addr = std::net::SocketAddrV6::new(dst, 0, 0, 0); // port = 0 on raw sockets
                    send_sock
                        .send_to(
                            &tcp_buffer,
                            &dst_addr.into(),
                        )
                        .expect("IPv6 raw send failed");

                    wait_reply_ipv6_raw(dst, self.port, start).await
                }
            };

            match reply {
                Some((latency, reply_flags)) => {
                    let flags_str = decode_tcp_flags(reply_flags);

                    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
                    disable_raw_mode()?;
                    println!(
                        "[{}] Reply in {:.3} ms - flags=0x{:02x} ({})",
                        i + 1,
                        latency.as_secs_f64() * 1000.0,
                        reply_flags,
                        flags_str
                    );
                    enable_raw_mode()?;

                    // Treat anything >= 5 s as a timeout
                    if latency.as_millis() as u16 >= 5000 {
                        self.data.push(5000);
                    } else {
                        self.data.push(latency.as_millis() as u16);
                    }
                    self.sys_time.push(self.get_time());

                    // Sampling window: every `scale` pings we emit one graph data point
                    if j == scale && opt_graph {
                        j = 0;
                        let scale_before = scale;

                        scale = match self.elapsed_time {
                            t if t <= 300     =>    5,
                            t if t <= 1800    =>   15,
                            t if t <= 3600    =>   30,
                            t if t <= 7200    =>   60,
                            t if t <= 14400   =>  120,
                            t if t <= 28800   =>  240,
                            t if t <= 57600   =>  480,
                            t if t <= 115200  =>  960,
                            t if t <= 230400  => 1920,
                            t if t <= 460800  => 3840,
                            t if t <= 921600  => 7680,
                            _                 => 15360,
                        };

                        // If the scale changed, resample all accumulated data at the new resolution
                        if scale_before != scale {
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

            // Interval between pings (~2 pings/sec by default)
            if self.nb_ping != 0 {
                if interval_ms > 0 && i < self.nb_ping - 1 {
                    sleep(Duration::from_millis(interval_ms)).await;
                }
            } else if interval_ms > 0 {
                sleep(Duration::from_millis(interval_ms)).await;
            }
        }

        use crossterm::terminal::disable_raw_mode;
        disable_raw_mode()?;
        self.latency_data();
        Ok(())
    }

    // Override output filename and ping count after construction.
    #[allow(dead_code)]
    fn setting(&mut self, output: &str, nb_ping: u16) {
        self.output = output.to_string();
        self.nb_ping = nb_ping;
    }

    // Construct a new TCPPingTool with all vectors initialised to empty.
    pub fn new(target: &str, output: &str, nb_ping: u16, port: u16, flag: u8) -> Self {
        Self {
            target: target.to_string(),
            output: output.to_string(),
            port,
            nb_ping,
            flag,
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
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IPv4 helpers
// ─────────────────────────────────────────────────────────────────────────────

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

// Layer3 channel: ipv4_packet_iter gives raw IPv4 packets; TCP must be extracted from payload.
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

// ─────────────────────────────────────────────────────────────────────────────
// IPv6 helpers — socket2 raw socket, arbitrary TCP flags supported
// ─────────────────────────────────────────────────────────────────────────────

// Build a TCP-only segment for IPv6 raw socket.
// The kernel adds the IPv6 header automatically on SOCK_RAW with IPPROTO_TCP.
// We must compute the TCP checksum manually using the IPv6 pseudo-header.
fn build_tcp_packet_v6(
    buffer: &mut [u8],
    src_ip: Ipv6Addr,
    dst_ip: Ipv6Addr,
    src_port: u16,
    dst_port: u16,
    flags: u8,
) {
    {
        let mut tcp = MutableTcpPacket::new(buffer).unwrap();
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
    // Checksum must be computed after all fields are set
    let cksum = tcp_checksum_ipv6(src_ip, dst_ip, buffer);
    let mut tcp = MutableTcpPacket::new(buffer).unwrap();
    tcp.set_checksum(cksum);
}

fn tcp_checksum_ipv6(src_ip: Ipv6Addr, dst_ip: Ipv6Addr, tcp_packet: &[u8]) -> u16 {
    let mut sum = 0u32;

    // IPv6 pseudo-header: src, dst, TCP length, next header (6 = TCP)
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

    // TCP segment
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

// Receive the TCP reply on a raw IPv6 socket.
// On Linux, raw IPv6 sockets with IPPROTO_TCP receive TCP segments directly
// (the IPv6 header is stripped by the kernel before delivery).
async fn wait_reply_ipv6_raw(
    target_ip: Ipv6Addr,
    target_port: u16,
    start: Instant,
) -> Option<(Duration, u8)> {
    let recv_sock = Socket::new(
        Domain::IPV6,
        Type::RAW,
        Some(Protocol::from(6)), // IPPROTO_TCP = 6
    )
    .expect("failed to create IPv6 raw receive socket (need root)");

    // Non-blocking poll with 100ms timeout so we can check the overall deadline
    recv_sock.set_read_timeout(Some(Duration::from_millis(100))).unwrap();

    let timeout = Duration::from_millis(5000);
    let mut buf = vec![std::mem::MaybeUninit::<u8>::uninit(); 1024];

    loop {
        if start.elapsed() > timeout {
            return Some((Duration::from_millis(5000), 0x00));
        }

        match recv_sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                // Filter by source address
                if let Some(std::net::IpAddr::V6(src_ip)) = addr.as_socket_ipv6().map(|a| IpAddr::V6(a.ip().clone())) {
                    if src_ip != target_ip {
                        continue;
                    }
                } else {
                    continue;
                }

                // Reconstruct a byte slice from MaybeUninit
                let received: Vec<u8> = buf[..n]
                    .iter()
                    .map(|b| unsafe { b.assume_init() })
                    .collect();

                if let Some(tcp) = pnet::packet::tcp::TcpPacket::new(&received) {
                    if tcp.get_source() == target_port {
                        return Some((start.elapsed(), tcp.get_flags()));
                    }
                }
            }
            Err(_) => continue, // timeout on this recv, loop and check overall deadline
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Common helpers
// ─────────────────────────────────────────────────────────────────────────────

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