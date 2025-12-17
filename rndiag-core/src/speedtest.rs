use crate::tool::ConnectTool;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Instant, Duration};

//SpeedTest object definition
pub struct SpeedTest{
    srv_addr: String, //addr to listen in server mode, or addr to contact in client mode
    srv_port: u16, //port to listen in server mode or port to contact in port mode
    is_srv: bool, //true => run as server, false => run as client
    tst_duration: u64, //Duration of the speedtest
    mbps: u64, //Bandwidth limit for the speedtest
    mode: String, //full => upload + Download, upload => upload only, download => download only
}

//Methods definition for methods inerhited by the trait
impl ConnectTool for SpeedTest{
    //Returne the name of the tool
    fn name(&self) -> &'static str {
        "SpeedTest"
    }

    //Return srv_addr attribute
    fn srv_addr(&self) -> &str {
        &self.srv_addr
    }

    #[allow(unused_variables)]
    //Main method
    async fn run(&mut self) -> std::io::Result<()> {
        //Server mode
        if self.is_srv == true {
            self.start_server().await?;
            return Ok(());
        }

        //Client mode
        let result = &self.client().await?;

        Ok(())
    }

    #[allow(unused_assignments)]
    //Start server and handle connexion
    async fn start_server(&mut self) -> std::io::Result<()> {
        let duration = self.tst_duration;

        //Need init the IpAddr object before use it, so init with 0.0.0.0 and will be modified later
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

        //Resolve if the user given a hostname
        if self.srv_addr.parse::<IpAddr>().is_ok(){
            target_ip = self.srv_addr.parse().expect("Invalid IP address");

        }
        else {
            target_ip = self.resolve();
        }

        let listener = TcpListener::bind((target_ip, self.srv_port)).await?;
        println!("Server listening on port {}", &self.srv_port);

        loop {
            let (mut socket, addr) = listener.accept().await?;
            println!("Client connected: {}", addr);

            let limit_bytes_per_sec = (self.mbps * 1024 * 1024 / 8) as usize;
            let mode = self.mode.clone();

            tokio::spawn(async move {
                //Depending the mode we launch full speedtest or only Download or only Upload
                if &mode == "full" {
                    // Upload and Download
                    if let Err(e) = handle_upload(&mut socket, limit_bytes_per_sec, duration).await {
                        eprintln!("Upload error: {}", e);
                        return;
                    }
                    println!("Upload test completed, starting download...");
                    
                    //Sleep for sync with the client
                    sleep(Duration::from_millis(500)).await;
                    
                    if let Err(e) = handle_download(&mut socket, limit_bytes_per_sec, duration).await {
                        eprintln!("Download error: {}", e);
                    }
                    println!("Download test completed");
                }
                else if &mode == "upload" {
                    //Launch only the upload
                    if let Err(e) = handle_upload(&mut socket, limit_bytes_per_sec, duration).await {
                        eprintln!("Upload error: {}", e);
                    }
                }
                else if &mode == "download" {
                    //Launch only the download
                    if let Err(e) = handle_download(&mut socket, limit_bytes_per_sec, duration).await {
                        eprintln!("Download error: {}", e);
                    }
                }
            });
        }
    }

    #[allow(unused_assignments)]
    //Handle the client side
    async fn client(&mut self) -> std::io::Result<()> {

        //Need init the IpAddr object before use it, so init with 0.0.0.0 and will be modified later
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

        //Resolve if the user given a hostname
        if self.srv_addr.parse::<IpAddr>().is_ok(){
            target_ip = self.srv_addr.parse().expect("Invalid IP address");
        }
        else {
            target_ip = self.resolve();
        }
        let target_string = target_ip.to_string();

        //Build string with addr of server + the dst port to have this format => 1.2.3.4:80
        let addr_port = target_string + ":" + self.srv_port.to_string().as_str();
        let mut socket = TcpStream::connect(addr_port).await?;
        println!("Connected to server {}:{}", &self.srv_addr, &self.srv_port);

        //Run upload if the mode is full or upload
        if &self.mode == "full" || &self.mode == "upload"{
            // ----------------- UPLOAD -----------------
            println!("Starting UPLOAD test...");

            let buffer = vec![0u8; 64 * 1024];
            let start = Instant::now();
            let mut total_sent = 0usize;

            let limit_bytes_per_sec = (self.mbps as usize * 1000000) / 8;
            let duration_secs = self.tst_duration;

            while start.elapsed() < Duration::from_secs(duration_secs) {
                let elapsed = start.elapsed().as_secs_f64();
                let expected_bytes = (limit_bytes_per_sec as f64) * elapsed;

                if (total_sent as f64) >= expected_bytes {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }

                let chunk_size = std::cmp::min(buffer.len(), limit_bytes_per_sec);
                socket.write_all(&buffer[..chunk_size]).await?;
                total_sent += chunk_size;
            }

            //End upload sign
            socket.write_all(b"UPLOAD_DONE\n").await?;
            socket.flush().await?;

            let elapsed = start.elapsed().as_secs_f64();
            let mbps = total_sent as f64 * 8.0 / 1_000_000.0 / elapsed;
            println!("Upload rate: {:.2} Mbps", mbps);
            
            //Sleep for sync with the server
            sleep(Duration::from_millis(500)).await;
        }

        //Run download if the mode is full or download
        if &self.mode == "full" || &self.mode == "download"{
            // ----------------- DOWNLOAD -----------------
            println!("Starting DOWNLOAD test...");
            let mut total_received = 0usize;
            let start = Instant::now();

            loop {
                let mut buf = vec![0u8; 64*1024];
                let n = socket.read(&mut buf).await?;
                if n == 0 { break; }
                total_received += n;

                if start.elapsed() >= Duration::from_secs(self.tst_duration) { 
                    break; 
                }
            }

            let elapsed = start.elapsed().as_secs_f64();
            let mbps = total_received as f64 * 8.0 / 1_000_000.0 / elapsed;
            println!("Download rate: {:.2} Mbps", mbps);
        }

        Ok(())
    }
}

//Specific methods of speedtest tool that not inerhited by the trait
impl SpeedTest{
    #[allow(dead_code)]
    //Setting attributes method
    fn setting (&mut self, srv_addr: &str, srv_port: u16, mode: &str, is_srv: bool, tst_duration: u64, mbps: u64){
        self.srv_addr = srv_addr.to_string();
        self.srv_port = srv_port;
        self.mode = mode.to_string();
        self.is_srv = is_srv;
        self.tst_duration = tst_duration;
        self.mbps = mbps;
    }

    //Init object method
    pub fn new(srv_addr: &str, srv_port: u16, mode: &str, is_srv: bool, tst_duration: u64, mbps: u64) -> Self {
        Self {
            srv_addr: srv_addr.to_string(),
            srv_port: srv_port,
            is_srv: is_srv,
            tst_duration: tst_duration,
            mbps: mbps,
            mode: mode.to_string(),
        }
    }
}

//Handle upload method for server part
async fn handle_upload(socket: &mut tokio::net::TcpStream, limit_bytes_per_sec: usize, duration_secs: u64) -> std::io::Result<()> {
    println!("Starting UPLOAD test...");
    let mut buffer = vec![0u8; 64 * 1024];
    let start = Instant::now();
    let mut total_uploaded = 0usize;

    loop {
        // Global Timeout
        if start.elapsed() >= Duration::from_secs(duration_secs + 2) {
            println!("Upload timeout reached");
            break;
        }

        tokio::select! {
            result = socket.read(&mut buffer) => {
                let n = result?;
                if n == 0 { 
                    println!("Client closed connection");
                    break; 
                }
                
                total_uploaded += n;

                //Check if we received the "UPLOAD_DONE" sign from the client
                if buffer[..n].windows(12).any(|w| w == b"UPLOAD_DONE\n") {
                    println!("Received UPLOAD_DONE signal");
                    break;
                }

                // Throttling
                let elapsed = start.elapsed().as_secs_f64();
                let expected_bytes = (limit_bytes_per_sec as f64) * elapsed;
                if total_uploaded as f64 > expected_bytes {
                    sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }
    //Time calculation + moy bandwidth calculation during the speedtest
    let elapsed = start.elapsed().as_secs_f64();
    let mbps = total_uploaded as f64 * 8.0 / 1_000_000.0 / elapsed;
    println!("Upload completed: {:.2} Mbps", mbps);

    Ok(())
}


//Handle download method for server part
async fn handle_download(socket: &mut tokio::net::TcpStream, limit_bytes_per_sec: usize, duration_secs: u64) -> std::io::Result<()> {
    println!("Starting DOWNLOAD test...");
    let buffer = vec![0u8; 64 * 1024];
    let start = Instant::now();
    let mut total_sent = 0usize;

    while start.elapsed() < Duration::from_secs(duration_secs) {
        let elapsed = start.elapsed().as_secs_f64();
        let expected_bytes = (limit_bytes_per_sec as f64) * elapsed;
        
        if total_sent as f64 >= expected_bytes {
            sleep(Duration::from_millis(10)).await;
            continue;
        }

        let chunk_size = std::cmp::min(buffer.len(), limit_bytes_per_sec);
        
        //Ignore errors if the client has closed the connexion
        match socket.write_all(&buffer[..chunk_size]).await {
            Ok(_) => total_sent += chunk_size,
            Err(_) => break,
        }
    }

    //Time calculation + moy bandwidth calculation during the speedtest
    let elapsed = start.elapsed().as_secs_f64();
    let mbps = total_sent as f64 * 8.0 / 1_000_000.0 / elapsed;
    println!("Download completed: {:.2} Mbps", mbps);
    
    Ok(())
}