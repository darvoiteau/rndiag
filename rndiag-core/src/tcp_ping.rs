use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crossterm::event::KeyModifiers;
use rndiag_graph::graph::graph_display;
use crossterm::event::{self, Event, KeyCode};
use std::net::IpAddr;

use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::{MutableIpv4Packet, checksum as ipv4_checksum};
use pnet::packet::tcp::MutableTcpPacket;
use pnet::packet::{Packet};
use pnet::transport::{
    transport_channel, TransportChannelType, ipv4_packet_iter,
};

use crate::tool::LatencyTool;

pub struct TCPPingTool {
    pub target: String, //IP or host to ping
    port: u16, //Port to ping with tcp ping
    data: Vec<u16>, //Latency of each pin
    begin_time: u64, //Time when pings start => Used to determine the elapsed time to detect scale for sampling
    elapsed_time: u64, //Elasped time since the first ping => Used with the begin_time to determine the scale for sampling
    sys_time: Vec<u64>, //Get the current system timestamp of each ping => used to get the number of data ping's and used to calculate elapsed_time
    latency_time: Vec<u64>, //Store the system timestamp of each sampled graph data point ping
    latency_min: Vec<u16>, //Store each latency ping => used for min latency sampling calculation
    latency_moy: Vec<u16>, //Store each latency ping => used for moy latency sampling calculation
    latency_max: Vec<u16>, //Store each latency ping => used for max latency sampling calculation
    latency_min_sampled: Vec<u64>, //Store sampled min latency values
    latency_moy_sampled: Vec<u64>, //Store sampled moy latency values
    latency_max_sampled: Vec<u64>, //Store sampled max latency values
    output: String, //Destination csv file
    nb_ping: u16, //The number of ping defined by the user or if default => infinity ping
    flag: u8,  // The TCP flag that will be used for launch the tcp ping
}

//Methods specifically defined for the PingTool object about the inherited NetworkTool Trait
impl LatencyTool for TCPPingTool {
    //Return only the name of the object
    fn name(&self) -> &'static str {
        "tping"
    }

    //Return the data vec => used in the definition of NetworkTool Methods trait when it wants to read object attribute
    fn data(&self) -> &Vec<u16> {
        &self.data
    }

    //Return the data vec => used in the definition of NetworkTool Methods trait when it wants to read object attribute
    fn nb_ping(&self) -> &u16{
        &self.nb_ping
    }

    //Return the sys_time vec => used in the definition of NetworkTool Methods trait when it wants to read object attribute
    fn sys_time(&self) -> &Vec<u64> {
        &self.sys_time
    }

    //Return the elapsed_time var => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn elapsed_time(&mut self) -> &mut u64 {
        &mut self.elapsed_time
    }

    //Return the begin_time var => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn begin_time(&self) -> &u64 {
        &self.begin_time
    }

    //Return the latency_time vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_time(&mut self) -> &mut Vec<u64> {
        &mut self.latency_time
    }

    //Return the latency_min vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_min(&mut self) -> &mut Vec<u16> {
        &mut self.latency_min
    }

    //Return the latency_moy vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_moy(&mut self) -> &mut Vec<u16> {
        &mut self.latency_moy
    }

    //Return the latency_max vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_max(&mut self) -> &mut Vec<u16> {
        &mut self.latency_max
    }

    //Return the latency_min_sampled vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_min_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_min_sampled
    }

    //Return the latency_moy_sampled vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_moy_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_moy_sampled
    }

    //Return the latency_max_sampled vec => used in the definition of NetworkTool Methods trait when it wants to read and modify object attribute
    fn latency_max_sampled(&mut self) -> &mut Vec<u64> {
        &mut self.latency_max_sampled
    }

    //Return the output attribute => filename in CSV of tool result
    fn output(&self) -> &str {
        &self.output
    }

    //Return the target attribute => destination that will be used to launch the tool
    fn target(&self) -> &str {
        &self.target
    }

    async fn run(&mut self) -> std::io::Result<()> {
    //Pour chaque flag faire un if de ce block mais en remplacant ACK, par SYN, FIN, ... pour chaque if/else if
    self.tcp_ping(self.flag,500/*interval entre chaque tcp ping*/).await?;

    Ok(())
}

}

//Specific methods of object that not inerhited of the trait
impl TCPPingTool{
#[allow(unused_assignments)]

//Main method to launch the tool: build packet with flag get the latency, ...
async fn tcp_ping(&mut self, flags: u8, interval_ms: u64) -> std::io::Result<()> {
    let mut i: u16 = 0;

    let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

    //used to do ping while is not equal to scale. 1 scale = 1 sampling for graph
    let mut j: u16 = 0;

    //Resolve if the user given a hostname
    if self.target.parse::<IpAddr>().is_ok(){
        target_ip = self.target.parse().expect("Invalid IP address");

    }
    else {
        target_ip = self.resolve();
    }

    
    //Gain source IP via false UDP connexion
    let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(format!("{}:80", self.target))?;
    let src_ip = match socket.local_addr()? {
        std::net::SocketAddr::V4(addr) => *addr.ip(),
        _ => panic!("Expected IPv4 address"),
    };
    
    let src_port: u16 = 54321;

    //Use Layer3 to get full control
    let protocol = TransportChannelType::Layer3(IpNextHeaderProtocols::Tcp);

    let (mut sender, mut receiver) = transport_channel(4096, protocol)
        .expect("failed to open raw socket (need root/admin privileges)");
    
    let flags_str = decode_tcp_flags(flags);
    println!(
        "TCP-PING {}:{} from {}:{} flags=0x{:02x} ({}) count={}",
        self.target, self.port, src_ip, src_port, flags, flags_str, self.nb_ping
    );

    //used to save the "progression" in vectors to sampling values of news pings only
    let mut k: usize = 0;

    //define the scale value for sampling
    let mut scale: u16 = 5;

    let opt_graph = true;

    self.begin_time = self.get_time();

    let target_v4: Ipv4Addr = match target_ip {
    IpAddr::V4(v4) => v4,
    _ => {
        eprintln!("Destination IPv6 non supportée");
        return Ok(());
    }
};
    //Main loop
    //Capture keyboard, launch tcp ping and get the latency of each tcp ping
    while i < self.nb_ping || self.nb_ping == 0 {
        let mut buffer = [0u8; 40]; // 20 IP + 20 TCP

        //Build the packet with the given flag
        build_packet(
            &mut buffer,
            src_ip,
            target_v4,
            src_port,
            self.port,
            flags,
        );


        //Enable raw terminal to capture correctly input from user
            use crossterm::terminal::enable_raw_mode;
            enable_raw_mode()?;
            if event::poll(Duration::from_millis(20))? {
                if let Event::Key(key_event) = event::read().unwrap() {
                    match (key_event.code, key_event.modifiers) {
                        (KeyCode::Char('g'), KeyModifiers::NONE) => {
                            // compute sampling before showing graph for latest data
                            k = self.sampling(k, scale);
                            // show graph (blocking UI). This will block this async task until closed.
                            graph_display(
                                &self.latency_min_sampled,
                                &self.latency_moy_sampled,
                                &self.latency_max_sampled,
                            ).unwrap_or_else(|e|{
                            eprintln!("Error during graph building: {}", e);
                            });
                        }
                        //If the user do Control C the program will be exit.
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            use crossterm::terminal::disable_raw_mode;
                            //Disable raw terminal to prevent displaying issue of println!
                            disable_raw_mode()?;
                            //Before quit the program => calcul statistics: min/avg/max and % of packet loss
                            self.latency_data();
                            return Ok(())

                        }
                        
                        _ => {}
                    }
                }
            }

        let start = Instant::now();

        //Send the complete packet (IP + TCP)
        sender
            .send_to(
                pnet::packet::ipv4::Ipv4Packet::new(&buffer).unwrap(),
                std::net::IpAddr::V4(target_v4)
            )
            .expect("send_to failed");

        //Wait for reply by the defined dst
        let reply = wait_reply(&mut receiver, target_v4, self.port, start).await;

        match reply {
            //If packet was build and sent correctly, calculation of the latency of one tcp ping
            Some((latency, reply_flags)) => {
                let flags_str = decode_tcp_flags(reply_flags);
                //Disable raw terminal to prevent displaying issue of println!    
                use crossterm::terminal::disable_raw_mode;
                disable_raw_mode()?;
                println!("[{}] Reply in {:.3} ms - flags=0x{:02x} ({})", i + 1, latency.as_secs_f64() * 1000.0, reply_flags, flags_str);
                enable_raw_mode()?;

                if latency.as_millis() as u16 >= 5000{
                    self.data.push(5000);
                    self.sys_time.push(self.get_time());

                    }
                else{
                    self.data.push(latency.as_millis() as u16);
                    self.sys_time.push(self.get_time());
                }

                //if j == scale we make a sampling of data => make 1 point in the graph
                if j == scale && opt_graph == true {
                    
                    j = 0;
                    let scale_changed: u16 = scale;

                    //Depending of the elapsed time the number of value (number of ping) for sampling is different.
                    if opt_graph == true && self.elapsed_time <= 300{
                    scale = 5;
                    }
                    else if opt_graph == true && self.elapsed_time <= 1800{
                        scale = 15;
                    }
                    else if opt_graph == true && self.elapsed_time <= 3600{
                        scale = 30;
                    }
                    else if opt_graph == true && self.elapsed_time <= 7200{
                        scale = 60;
                    }
                    else if opt_graph == true && self.elapsed_time <= 14400{
                        scale = 120;
                    }
                    else if opt_graph == true && self.elapsed_time <= 28800{
                        scale = 240;
                    }
                    else if opt_graph == true && self.elapsed_time <= 57600{
                        scale = 480;
                    }
                    else if opt_graph == true && self.elapsed_time <= 115200{
                        scale = 960;
                    }
                    else if opt_graph == true && self.elapsed_time <= 230400{
                        scale = 1920;
                    }
                    else if opt_graph == true && self.elapsed_time <= 460800{
                        scale = 3840;
                    }
                    else if opt_graph == true && self.elapsed_time <= 921600{
                        scale = 7680;
                    }
                    else if opt_graph == true && self.elapsed_time <= 1843200{
                        scale = 15360;
                    }

                    //Detect if the scale value has changed => if its the case we sampling again all ping latency data for the new scale
                    if scale_changed != scale{
                        k = 0;

                        //For each new scale we clear sampled data to re-sample again all current data but in the new scale of sampling.
                        self.latency_max_sampled.clear();
                        self.latency_moy_sampled.clear();
                        self.latency_min_sampled.clear();

                        //Send 0 as scale to detect in sampling() that we have changed the scale to sampling again values for this scale
                        k = self.sampling(k, 0);
                    }
                    k = self.sampling(k, scale);

                }
                //Incremente j for each loop trip except if j == scale
                else if opt_graph == true {
                    j+=1;
                }
                i+=1;

            }
            //If the packet is malformated or not correctly sent, stop the program with an error
            None =>{
                panic!("Inconsistent data in packet response");

            }
        }

        //Wait when it needed between each tcp_ping to respect the defined interval_ms between each tcp ping
        if self.nb_ping != 0 {

            //If nb_ping == 0 the program will panic because i cannot be inferior to 0
            if interval_ms > 0 && i < self.nb_ping - 1 {
                sleep(Duration::from_millis(interval_ms)).await;
            }
        }
        else {
            //But we need wait even if nb_ping == 0, this is the reason of this if/else
            if interval_ms > 0 {
                sleep(Duration::from_millis(interval_ms)).await;
            }
        }
    }

    //Disable raw terminal to prevent displaying issue of println!    
    use crossterm::terminal::disable_raw_mode;
    disable_raw_mode()?;

    self.latency_data();
    Ok(())
}

}

//Method definition decode_tcp_flags
//Depending the received tcp flag value by the dst, we decode it to be human redeable
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

//Method that manage the tcp ping response from the dst
async fn wait_reply(receiver: &mut pnet::transport::TransportReceiver, target_ip: Ipv4Addr, target_port: u16, start: Instant) -> Option<(Duration, u8)> {
    let timeout = Duration::from_millis(5000);

    loop {
        
        if start.elapsed() > timeout {
            return Some((timeout, 0));
        }

        // Create IPV4 iterator
        let mut iter = ipv4_packet_iter(receiver);
        
        match iter.next() {
            Ok((packet, addr)) => {
                
                // Check it that our IP
                if let std::net::IpAddr::V4(src_ip) = addr {
                    if src_ip != target_ip {
                        continue;
                    }
                } else {
                    continue;
                }

                // Parse the tcp packet from the IPv4 payload
                if packet.get_next_level_protocol() == IpNextHeaderProtocols::Tcp {
                    if let Some(tcp) = pnet::packet::tcp::TcpPacket::new(packet.payload()) {
                        if tcp.get_source() == target_port {
                            return Some((start.elapsed(), tcp.get_flags()));
                        }
                    }
                }
            }
            Err(_) => {
                // Wait before to attempt
                sleep(Duration::from_millis(10)).await;
                continue;
            }
        }
    }
}

//Method to build tcp packet
fn build_packet(buffer: &mut [u8], src_ip: Ipv4Addr, dst_ip: Ipv4Addr, src_port: u16, dst_port: u16, flags: u8) {
    //Build IPv4 header
    let mut ip_packet = MutableIpv4Packet::new(buffer).unwrap();
    ip_packet.set_version(4);
    ip_packet.set_header_length(5); // 5 * 4 = 20 bytes
    ip_packet.set_total_length(40); // 20 (IP) + 20 (TCP)
    ip_packet.set_ttl(64);
    ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ip_packet.set_source(src_ip);
    ip_packet.set_destination(dst_ip);
    ip_packet.set_checksum(0);
    
    // IP checksum calculation
    let checksum = ipv4_checksum(&ip_packet.to_immutable());
    ip_packet.set_checksum(checksum);

    // Build the TCP header in the IP payload
    let tcp_buffer = &mut buffer[20..40];
    {
        let mut tcp = MutableTcpPacket::new(tcp_buffer).unwrap();
        
        tcp.set_source(src_port);
        tcp.set_destination(dst_port);
        tcp.set_sequence(12345);
        tcp.set_acknowledgement(0);
        tcp.set_data_offset(5); // 5 * 4 = 20 bytes
        tcp.set_flags(flags);
        tcp.set_window(64240);
        tcp.set_urgent_ptr(0);
        tcp.set_checksum(0);
    } //TCP is dropped here. The mutable borrow is free

    //TCP Checksum calculation
    let cksum = tcp_checksum(src_ip, dst_ip, &buffer[20..40]);
    
    // Create again the MutableTCPPacket to set the checksum
    let mut tcp = MutableTcpPacket::new(&mut buffer[20..40]).unwrap();
    tcp.set_checksum(cksum);
}

//Checksum calculation method
fn tcp_checksum(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    tcp_packet: &[u8],
) -> u16 {
    let mut sum = 0u32;

    // Pseudo-header
    for byte in src_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    for byte in dst_ip.octets().chunks(2) {
        sum += u16::from_be_bytes([byte[0], byte[1]]) as u32;
    }
    sum += IpNextHeaderProtocols::Tcp.0 as u32;
    sum += tcp_packet.len() as u32;

    // TCP packet
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

//Specific Object method that not inerithed of the trait
impl TCPPingTool{
    #[allow(dead_code)]
    //Method to set attribute object
    fn setting (&mut self, output: &str, nb_ping: u16){
        self.output = output.to_string();
        self.nb_ping = nb_ping;

    }

    //Method to init the object
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