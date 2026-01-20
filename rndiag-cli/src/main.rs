use std::thread::sleep;
use rndiag_core::nslookup::NSlookup;
use argh::FromArgs;
use rndiag_core::ping::{PingTool};
use rndiag_core::speedtest::SpeedTest;
use rndiag_core::tcp_message;
use rndiag_core::tcp_ping::TCPPingTool;
use rndiag_core::tool::LatencyTool;
use rndiag_core::tool::ConnectTool;
use tokio;
use tokio::time::Duration;
use rndiag_server::{self, web_server};

mod sanitizer;
mod diagnostic;


#[derive(FromArgs)]
///reach new args
struct Args{
    #[argh(option, short = 'd', default=r#"String::from("none")"#)]
    ///destination server ip or name
    dst: String,

    #[argh(option, short = 'c', default = "0")]
    ///stop after <count> replies
    count: u16,

    #[argh(option, short = 'o', default = r#"String::from("AjaNuP123YuL903nNNaZY")"#)]
    ///output csv filename
    output: String,

    #[argh(option, short = 'p', default = "0")]
    ///destination port
    port: u16,

    #[argh(option, short = 'm', default = r#"String::from("full")"#)]
    ///mode for speedtest, upload => upload, download => download, full => upload + download
    mode: String,

    #[argh(option, short = 's', default = "false")]
    ///for tools in server-client mode, true => run as server, false => run as client, default => false
    server: bool,

    #[argh(option, short='t', default = "30")]
    ///speedtest duration in secs.
    time: u64,

    #[argh(option, short='b', default = "50000")]
    ///target bitrate in Mbps, default 0 for unlimited
    bitrate: u64,

    #[argh(option, short='f', default = r#"String::from("none")"#)]
    ///tcp flag for tcp_ping. S => SYN, A => ACK, R => RST, F => FIN, P => PUSH, U => URG
    flag: String,

    #[argh(option, short = 'D', default = r#"String::from("none")"#)]
    ///quick network diagnostics (ping latency, resolution latency, tcp_ping). to use diagnostic -D => True 
    ///Usage: rndiag -D <speedtestSrv> -d => specify specific server to resolve and to contact for ping and tcp_ ping, -p => specify specific port to contact for tcp_ping
    diagnostic: String,

    #[argh(option, short='P', default = "false")]
    ///to use ping, -P => true + specify destination -d
    ping: bool,
    
    #[argh(option, short='T', default = "false")]
    ///to use tcp ping, -T => true + specify destination -d and port -p
    tping: bool,

    #[argh(option, short='R', default = "false")]
    ///to use DN resolver, -R => true + specify the server to resolve -d
    resolver: bool,

    #[argh(option, short='S', default = "false")]
    ///client-server tool,to launch it on client side, -S + specify the server -d + specify the port -p + specify mode -m. 
    ///on server side -S + -s true + specify the listening addr -d + the listening port -p
    sptest: bool,

    #[argh(option, short='N', default = "false")]
    ///client-server tool, to launch it on client side, -S + specify the server -d + specify the port -p. 
    /// on server side -S + -s true + specify the listening addr -d + the listening port -p
    nc: bool,

    #[argh(option, default = "false")]
    /// provide a web-page with metrics tht can be scrapped by prometheus/grafana, --exporter true
    exporter: bool,

    #[argh(option, default = r#"String::from("none")"#)]
    /// IP of the web-server for exporter mode
    ws_addr: String,

    #[argh(option, default = "0")]
    /// port of the web-server for exporter mode
    ws_port: u16,

}
#[allow(unused_assignments)]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    //Create options struct and capture options given by the user
    let options: Args = argh::from_env();
    let mut flag_u8: u8 = 0;

    //Check what tool is selected by the user
    let selected_tool = sanitizer::tool_is_selected(&options);

    //Check if needed option depending of the tool is correctly set by the user
    sanitizer::correct_options(&selected_tool, &options);
    //Warn the user if useless option depending of the tool are set
    sanitizer::useless_options(&selected_tool, &options);

    //We check later the addr given by the user if the user choose diagnostic
    //We cannot check here the addr given by the user because if it is not the case rndiag set a default destination
    if selected_tool != "diagnostic" {
        sanitizer::addr_check(&options.dst);

    }    
    //If exporter option is chosen by the user
    if options.exporter == true {
        sanitizer::addr_check(&options.dst);
        
        let flag: String = String::from("S");

        flag_u8 = sanitizer::flag_format(&flag).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid TCP flag"))?;

        
        web_server::launch_srv(120, &options.dst, &options.ws_addr, options.port, options.ws_port, &options.output, &options.output, 6, flag_u8).await;

    }
    else if selected_tool == "ping" {
        //Sanitization + data conformity checking
        sanitizer::addr_check(&options.dst);
        sanitizer::output_check(&options.output);

        //Create PingTool object and init it with the new() method
        let mut ping_tool = PingTool::new(&options.dst, &options.output, options.count);

        //Run the PingTool object
        ping_tool.run().await.unwrap_or_else(|e|{
            eprintln!("Error during rndiag launching: {}", e);
        });

        //If the user was defined something for output it means he want to have an output
        if &options.output != "AjaNuP123YuL903nNNaZY"
        {
            //Call export_csv method inerhited of the trait of the object to save results in csv file
            if let Err(e) = ping_tool.export_csv() {
                eprintln!("Export CSV error: {}", e);
            }
        }

    }
    else if selected_tool == "resolver" {
        sanitizer::addr_check(&options.dst);
        sanitizer::output_check(&options.output);
        let mut nslookup_tool = NSlookup::new(&options.dst, &options.output, options.count);

        nslookup_tool.run().await.unwrap_or_else(|e|{
            eprintln!("Error during rndiag launching: {}", e);
        });

        
        if &options.output != "AjaNuP123YuL903nNNaZY"
        {
            if let Err(e) = nslookup_tool.export_csv() {
                eprintln!("Erreur d'export CSV : {}", e);
            }
        }

    }
    else if selected_tool == "tping" {
        sanitizer::addr_check(&options.dst);
        sanitizer::flag_check(&options.flag);
        sanitizer::output_check(&options.output);
        flag_u8 = sanitizer::flag_format(&options.flag).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid TCP flag"))?;

        let mut tcpping = TCPPingTool::new(&options.dst, &options.output, options.count, options.port, flag_u8);
        tcpping.run().await.unwrap_or_else(|e|{
            eprintln!("Error during rndiag launching: {}", e);
        });

        if &options.output != "AjaNuP123YuL903nNNaZY"
        {
            if let Err(e) = tcpping.export_csv() {
                eprintln!("Erreur d'export CSV : {}", e);
            }
        }

    }
    else if selected_tool == "sptest" {
        sanitizer::addr_check(&options.dst);
        sanitizer::mode_check(&options.mode);

        let mut speed_test = SpeedTest::new(&options.dst, options.port, &options.mode, options.server, options.time, options.bitrate);
        speed_test.run().await.unwrap_or_else(|e|{
            eprintln!("Error during rndiag launching: {}", e);
        });
    }
    else if selected_tool == "nc" {
        sanitizer::addr_check(&options.dst);

        let mut nc = tcp_message::TCPMessage::new(options.dst, options.port, options.server);
        nc.run().await.unwrap_or_else(|e|{
            eprintln!("Error during rndiag launching: {}", e);
        });
        
    }
    else if selected_tool == "diagnostic" {
        if options.dst == "none" {
            //If the user dosen't given the IP + Address, we set a dst addr and dst port by default (cloudflare) + flag by default
            let dst = "1.1.1.1".to_string();
            let port = 443;
            let flag = "S".to_string();

            //Get the flag number because tcp_ping need the flag number
            flag_u8 = sanitizer::flag_format(&flag).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid TCP flag"))?;


            //Create objects that will be used for the diagnostic
            let mut dping = PingTool::new(&dst, "none", 6);
            let mut dtping = TCPPingTool::new(&dst, "none", 6, port, flag_u8);
            let mut dresolver = NSlookup::new(&dst, "none", 6);

            //Run each object tool
            dping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });
            sleep(Duration::from_millis(1000));
            dtping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });
            sleep(Duration::from_millis(1000));
            dresolver.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });


            let mut zero_vec: Vec<u8> = Vec::new();
            let mut latency_max_vec: Vec<u8> = Vec::new();

            //Store the number of packet received of each object tool
            zero_vec.push(diagnostic::packet_received(&dping.data()));
            zero_vec.push(diagnostic::packet_received(&dtping.data()));
            zero_vec.push(diagnostic::packet_received(&dresolver.data()));

            //Store each latency moy sampled during the execution of each tools (6 pings currently)
            latency_max_vec.push(diagnostic::packet_latency(&dping.latency_moy_sampled()[0]));
            latency_max_vec.push(diagnostic::packet_latency(&dtping.latency_moy_sampled()[0]));
            latency_max_vec.push(diagnostic::packet_latency(&dresolver.latency_moy_sampled()[0]));

            //Call the function that format correctly in cli diagnostic result
            diagnostic::output_format(&latency_max_vec, &zero_vec);


            
        }
        else {
            //The case if the user given addr + port
            sanitizer::addr_check(&options.dst);

            //The flag is also set by default
            let flag: String = String::from("S");

            flag_u8 = sanitizer::flag_format(&flag).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid TCP flag"))?;

            let mut dping = PingTool::new(&options.dst, "none", 6);
            let mut dtping = TCPPingTool::new(&options.dst, "none", 6, options.port, flag_u8);
            let mut dresolver = NSlookup::new(&options.dst, "none", 6);

            dping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });
            sleep(Duration::from_millis(1000));
            dtping.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });
            sleep(Duration::from_millis(1000));
            dresolver.run().await.unwrap_or_else(|e|{
                eprintln!("Error during rndiag launching: {}", e);
            });

            let mut zero_vec: Vec<u8> = Vec::new();
            let mut latency_max_vec: Vec<u8> = Vec::new();

            zero_vec.push(diagnostic::packet_received(&dping.data()));
            zero_vec.push(diagnostic::packet_received(&dtping.data()));
            zero_vec.push(diagnostic::packet_received(&dresolver.data()));

            latency_max_vec.push(diagnostic::packet_latency(&dping.latency_max_sampled()[0]));
            latency_max_vec.push(diagnostic::packet_latency(&dtping.latency_max_sampled()[0]));
            latency_max_vec.push(diagnostic::packet_latency(&dresolver.latency_max_sampled()[0]));

            diagnostic::output_format(&latency_max_vec, &zero_vec);

        }
        

    }
    Ok(())
}

