use crate::metrics::MetricsLatency;

//resolver metrics object definition
pub struct NSLookupMetrics {
    latency_moy_sampled: u64, //Store the moy latency of last pings of equivalent tool object
    latency_level: u8, //Store the status of the latency by level. 0 => OK, 1 => Warning, 2 => Critical
    dst: String, //Store the dst to display it on metrics
    output_exporter: Vec<String>, //Contain formated as prom text format text ready to be write in metric file
}
//Methods specifically defined for the TPingMetrics object about the inherited NetworkTool Trait
impl MetricsLatency for NSLookupMetrics {
    //Return only the name of the object
    fn name(&self) -> &'static str {
        "resolver"
    }

    //Return the moy latency attribute
    fn latency_moy_sampled(&self) -> u64 {
        self.latency_moy_sampled
    }

    //Return the latency status/level
    fn latency_level(&mut self) -> &mut u8 {
        &mut self.latency_level
    }

    //Return the dst
    fn dst(&self) -> String {
        self.dst.clone()
    }

    //Return the content of output_exporter (formated prometheus format text)
    fn output_exporter(&mut self) -> &mut Vec<String> {
        &mut self.output_exporter
    }

    //Main method of the object
    fn run(&mut self) -> anyhow::Result<(), std::io::Error> {
        //Calling two methods defined in the Trait
        self.packet_latency();
        self.output_format();
        
        Ok(())
    }
}

//Specific method that not inerhited by the trait for this object
impl NSLookupMetrics{
    //Init object method
    pub fn new(latency_moy_sampled: u64, dst: String) -> Self{
        Self { latency_moy_sampled: latency_moy_sampled, dst: dst, latency_level: 0, output_exporter: Vec::new()}

    }
}