use std::net::IpAddr;
use regex::Regex;

use crate::Args;


//Return the selected tool, if no one is selected the program is stopped
pub fn tool_is_selected(options: &Args) -> String{
    let mut is_selected: u8 = 0;

    let mut selected_tool = String::new();

    if options.exporter == true {
        is_selected +=1;
        selected_tool = "exporter".to_string();
    }

    if options.ping == true{
        is_selected +=1;
        selected_tool = "ping".to_string();
    }

    if options.tping == true{
        is_selected +=1;
        selected_tool = "tping".to_string();
    }

    if options.resolver == true{
        is_selected +=1;
        selected_tool = "resolver".to_string();
    }

    if options.sptest == true{
        is_selected +=1;
        selected_tool = "sptest".to_string();
    }

    if options.nc == true {
        is_selected +=1;
        selected_tool = "nc".to_string();
    }


    if is_selected == 0{
        if options.diagnostic == "none" {
            eprintln!("Error ! Please select one tool: ping, tcp_ping, resolver, speedtest or nc");
            std::process::exit(1);
        }
        else {
            String::from("diagnostic")
        }
    }
    else if is_selected > 1{
        eprintln!("Error ! Please select only one tool !");
        std::process::exit(1);
    }
    else{
        selected_tool
    }
   

}

//Check if needed options are set by the user depending of the tool
pub fn correct_options(selected_tool: &String, options: &Args){
    if options.exporter == true {
        if options.port == 0 || options.dst == "none" || options.ws_addr == "none" || options.output == "output.csv" || options.ws_port == 0 {
            eprintln!("Error ! The destination IP/Hostname, the destination port for the target/exporter web-server, and the output file is required for exporter mode");
            std::process::exit(1);
        }
    }

    if selected_tool == "tping" || (selected_tool == "diagnostic" && options.dst != "none") {
        if options.port == 0{
            eprintln!("Error ! You must specify the destination port number for tping");
            std::process::exit(1);
        }

    }
    
    if selected_tool == "sptest" {
        if options.port == 0{
            eprintln!("Error ! You must sepcify the destination port number for sptest");
            std::process::exit(1);
        }
    }

    if selected_tool == "nc" {
         if options.port == 0{
            eprintln!("Error ! You must sepcify the destination port number for sptest");
            std::process::exit(1);
        }

    }
}

//Check if depending of the tool somes options given by the user are useless and if it the case, inform the user that rndiag will ignore it
pub fn useless_options(selected_tool: &String, options: &Args) {
    if (selected_tool == "ping" || selected_tool == "resolver") && options.port != 0 {
        println!("Warning ! The port number is no needed for this tool. This parameter will be ignored");
    }

    if selected_tool != "tping" && options.flag != "none" {
        println!("Warning ! The flag is no needed for this tool. This parameter will be ignored");
    }

    if selected_tool != "sptest" && (options.mode != "full" || options.time != 30 || options.bitrate != 50000) {
        println!("Warning ! time, bitrate, mode, are options that only available for speedtest. It will be ignored");
    }

    if selected_tool == "diagnostic" && (options.ping == true || options.resolver == true || options.tping == true || options.sptest == true || options.nc == true) {
        println!("Warning ! With diagnostic, you cannot select another tool. The selected tool will be ignored and not be runned");
    }

    if (selected_tool != "sptest" && selected_tool != "nc") && options.server == true {
        println!("Warning ! The server option is no needed for this tool. This parameter will be ignored");
    }
}

//Check if the addr option given by the user is a conform IP or a conform hostname
pub fn addr_check(addr: &String) {
    let is_addr: bool;


    if addr == "none" {
        is_addr = false;
    }
    else if addr.parse::<IpAddr>().is_ok() {
        is_addr = true;
    }
    else if addr.is_empty() || addr.len() > 253 {
        is_addr = false;
    }
    else if addr.split('.').all(|label| {!label.is_empty() && label.len() <= 63 && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') && !label.starts_with('-') && !label.ends_with('-')}){
        is_addr = true;
    }
    else{
        is_addr = false;
    }

    if is_addr == false{
        eprintln!("Error! The given addr is not a valid ip address or a valid hostname");
        std::process::exit(1);
    }
}

//Return the right flag number to use it with tcp_ping tool
pub fn flag_format(flag: &String) -> Option<u8> {
    match flag.to_uppercase().as_str() {
        "S" => Some(pnet::packet::tcp::TcpFlags::SYN),
        "A" => Some(pnet::packet::tcp::TcpFlags::ACK),
        "F" => Some(pnet::packet::tcp::TcpFlags::FIN),
        "R" => Some(pnet::packet::tcp::TcpFlags::RST),
        "P" => Some(pnet::packet::tcp::TcpFlags::PSH),
        "U" => Some(pnet::packet::tcp::TcpFlags::URG),
        _ => None,
    }
}
//Check if the given flag by the user is conform
pub fn flag_check(flag: &String) {
    if flag != "S" && flag != "A" && flag != "F" && flag != "F" && flag != "R" && flag != "P" && flag != "U" {
        eprintln!("Error ! Do not recognize the specified flag !");
        std::process::exit(1);
    }
}

//Sanitize the name of the output csv file
pub fn output_check(output: &String) {
    let unwaned_special_chars_re = Regex::new(r#"[=:*?"',;!{}\[\]()'<>|]+"#).unwrap();
    if unwaned_special_chars_re.is_match(output) {
        eprintln!("Error ! The given filename '{}' contain invalid special character", output);
        std::process::exit(1);
    }
}

//Check if the given mode by the user is conform
pub fn mode_check(mode: &String) {
    if mode != "full" && mode != "upload" && mode != "download" {
        eprintln!("Error ! Do not recognize the specified mode: '{}'", mode);
        std::process::exit(1);
    }
}