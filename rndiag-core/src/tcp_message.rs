use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use tokio;

use crate::tool::ConnectTool;

//TCPMessage object definition
pub struct TCPMessage{
    srv_addr: String, //addr to listen in server mode, or addr to contact in client mode
    srv_port: u16, //port to listen in server mode or port to contact in port mode
    is_srv: bool, //true => run as server, false => run as client
}
//Init object method
impl TCPMessage {
    pub fn new(srv_addr: String, srv_port: u16, is_srv: bool) -> Self {
        Self {
            srv_addr,
            srv_port,
            is_srv,
        }
    }

    //Handle bi-dir connexion (read + write)
    fn handle_connection(stream: TcpStream) -> io::Result<()> {
        //Clone the stream for the read thread
        let read_stream = stream.try_clone()?;
        let mut write_stream = stream;

        //Thread to receive messages
        let receive_thread = thread::spawn(move || {
            let mut reader = BufReader::new(read_stream);
            let mut line = String::new();

            
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        //Connexion closed
                        println!("\n[Connection closed by remote peer]");
                        std::process::exit(0);
                    }
                    Ok(_) => {
                        //Show the received message
                        print!("\rRemote: {}", line);
                        print!("You: ");
                        io::stdout().flush().unwrap();
                    }
                    Err(e) => {
                        eprintln!("\n[Error reading from connection: {}]", e);
                        std::process::exit(1);
                    }
                }
            }
        });

        //Main thread to send messages
        let stdin = io::stdin();
        let mut input = String::new();

        loop {
            print!("You: ");
            io::stdout().flush()?;

            input.clear();

            match stdin.read_line(&mut input) {
                Ok(0) => {
                    // EOF (Ctrl+D)
                    println!("[Closing connection...]");
                    break;
                }
                Ok(_) => {
                    //Send the message
                    if let Err(e) = write_stream.write_all(input.as_bytes()) {
                        eprintln!("[Error sending message: {}]", e);
                        break;
                    }
                    if let Err(e) = write_stream.flush() {
                        eprintln!("[Error flushing stream: {}]", e);
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("[Error reading input: {}]", e);
                    break;
                }
            }
        }

        //Wait the end of execution of the reception thread
        let _ = receive_thread.join();

        Ok(())
    }
}

//Methods definition that inerithed of the trait
impl ConnectTool for TCPMessage {
    //Return the tool name
    fn name(&self) -> &'static str {
        "TCPMessage"
    }

    //Return the srv_addr attribute
    fn srv_addr(&self) -> &str {
        &self.srv_addr
    }

    //Main method
    async fn run(&mut self) -> std::io::Result<()> {
        // Mode serveur
        if self.is_srv {
            self.start_server().await?;
            return Ok(());
        }
        // Mode client
        self.client().await?;
        Ok(())
    }

    #[allow(unused_assignments)]
    //Start server method => handle the server
    async fn start_server(&mut self) -> std::io::Result<()> {
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

        //Resolve if the user given a hostname
        if self.srv_addr.parse::<IpAddr>().is_ok(){
            target_ip = self.srv_addr.parse().expect("Invalid IP address");

        }
        else {
            target_ip = self.resolve();
        }

        let addr = format!("{}:{}", target_ip, self.srv_port);
        println!("[{}] Server listening on {}...", self.name(), addr);
        println!("Waiting for client connection...");

        //Use tokio::task::spawn_blocking for the blocking code
        let srv_port = self.srv_port;
        tokio::task::spawn_blocking(move || {
            let listener = TcpListener::bind(format!("{}:{}", target_ip, srv_port))?;
            let (stream, addr) = listener.accept()?;
            println!("Client connected from: {}", addr);
            println!("Type messages and press Enter to send. Ctrl+C to quit.\n");

            TCPMessage::handle_connection(stream)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;

        Ok(())
    }

    #[allow(unused_assignments)]
    //client method => Handle client part
    async fn client(&mut self) -> std::io::Result<()> {
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));

        //Resolve if the user given a hostname
        if self.srv_addr.parse::<IpAddr>().is_ok(){
            target_ip = self.srv_addr.parse().expect("Invalid IP address");

        }
        else {
            target_ip = self.resolve();
        }

        let addr = format!("{}:{}", target_ip, self.srv_port);
        println!("[{}] Connecting to {}...", self.name(), addr);

        //Use tokio::task::spawn_blocking for the blocking code
        let addr_clone = format!("{}:{}", target_ip, self.srv_port);
        tokio::task::spawn_blocking(move || {
            let stream = TcpStream::connect(&addr_clone)?;
            println!("Connected to server!");
            println!("Type messages and press Enter to send. Ctrl+C to quit.\n");

            TCPMessage::handle_connection(stream)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;

        Ok(())
    }
}
