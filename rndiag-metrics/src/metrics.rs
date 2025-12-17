use anyhow::Result;
use std::io::{self};

//NetworkTool trait => The trait for all networktool (ping, DNS resolving, telnet connection, ...)
pub trait MetricsLatency {
    fn name(&self) -> &'static str; //Get the name of the object
    fn latency_moy_sampled(&self) -> u64; //Get the object latency_moy_sampled attribute
    fn latency_level(&mut self) -> &mut u8; //Get the latency level object attribute
    fn dst(&self) -> String; //Get the dst object attribute
    fn output_exporter(&mut self) -> &mut Vec<String>; //Get the output_exporter object attribute
    fn run(&mut self) -> Result <(), io::Error>; //Run method

    //packet_latency method definition
    //Define the latency status/level with the provided moy latency of lasts pings tests with equivalent tools objects
    fn packet_latency(&mut self){
        
        //Latencies threshold in ms
        let latency1: u64 = 59;
        let latency2: u64 = 130;

        if self.latency_moy_sampled() >= latency2{
            *self.latency_level() = 2; // => critical status

        }
        else if self.latency_moy_sampled() >= latency1 {
            *self.latency_level() = 1; // => warning status
        }
        else {
            *self.latency_level() = 0; // => Ok status
        }
    }

    //output_format method definition
    //Format in prometheus format text metrics + needed infos (HELP and TYPE) and store it in output_exporter vec
    fn output_format(&mut self) {

        //------------------Latencies metrics format------------------
        let mut help_latency = String::from("# HELP ");
        let mut type_latency = String::from("# TYPE ");

        help_latency = help_latency + self.name() + " in ms\n";

        type_latency = type_latency + self.name() + " gauge\n";

        let mut metrics_latency: String = String::from(self.name());

        metrics_latency = metrics_latency + "{target=\"{}\"} " + self.latency_moy_sampled().to_string().as_str() + "\n";

        //------------------Latencies status metrics format------------------

        let mut help_latency_state = String::from("# HELP ");
        let mut type_latency_state = String::from("# TYPE ");

        help_latency_state = help_latency_state + self.name() + " state: 0 OK, 1 Warning, 2 NOK\n";
        type_latency_state = type_latency_state + self.name() + " gauge\n";
        
        let mut metrics_latency_state: String = String::from(self.name());
        metrics_latency_state = metrics_latency_state + "_state" + "{target=\"{}\"} " + self.latency_level().to_string().as_str() + "\n";


        //Push formated prom text data in output_exporter to be ready to write in metric file by web_server.rs
        self.output_exporter().push(help_latency);
        self.output_exporter().push(type_latency);
        self.output_exporter().push(metrics_latency);
        self.output_exporter().push(help_latency_state);
        self.output_exporter().push(type_latency_state);
        self.output_exporter().push(metrics_latency_state);
    }

    
}