use anyhow::Result;
use crossterm::event::KeyModifiers;
use rndiag_graph::graph::graph_display;
use tokio::task;
use crate::tool::LatencyTool;
use std::time::Duration;
use std::time::Instant;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::io::{self};
use tokio::time::sleep;
use crossterm::event::{self, Event, KeyCode};

//Ping object definition
pub struct PingTool {
    pub target: String, //IP or host to ping
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
    output: String, //Output CSV filename
    nb_ping: u16, //The number of ping defined by the user or if default => infinity ping
}

//Methods specifically defined for the PingTool object about the inherited NetworkTool Trait
impl LatencyTool for PingTool {
    //Return only the name of the object
    fn name(&self) -> &'static str {
        "ping"
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

    //Return the output csv filename
    fn output(&self) -> &str {
        &self.output
    }

    //Return the target attribute
    fn target(&self) -> &str {
        &self.target
    }

    //The run function of the ping object
    //Async because we have the normal continous ping task, and the keyboard key capture task for graph or quit the program
    #[allow(unused_assignments)]
    async fn run(&mut self) -> Result <(), io::Error> {
        let mut target_ip: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
        let nb_ping: u16 = self.nb_ping;

        //Resolve if the user given a hostname
        if self.target.parse::<IpAddr>().is_ok(){
            target_ip = self.target.parse().expect("Invalid IP address");

        }
        else {
            target_ip = self.resolve();
        }

        //used to do ping while i is < nb_ping
        let mut i: u16 = 0;
        
        //used to do ping while is not equal to scale. 1 scale = 1 sampling for graph
        let mut j: u16 = 0;

        //used to save the "progression" in vectors to sampling values of news pings only
        let mut k: usize = 0;

        //define the scale value for sampling
        let mut scale: u16 = 5;

        let opt_graph = true;
        

        self.begin_time = self.get_time();
        
        
        //Run this loop while the number of defined executed ping is not exceeded or in infinity
        while i < nb_ping || nb_ping == 0{
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
            
            //Start to count the time
            let start = Instant::now();
            //Async ping task => Async because we have this task + key input detection task above in same time
            let ping_result = task::spawn_blocking(move || {
            
            // Using a RAW socket (may require privileges)
            // Do a ping
            let res = match ping::new(target_ip)
                .socket_type(ping::RAW)
                .timeout(Duration::from_secs(5))
                .send()
            {
                Ok(_) => {},
                Err(e) => eprintln!("Ping failed with RAW socket: {}", e),
            };
            res
            }).await;
            
            ping_result.unwrap_or_else(|e|{
                eprintln!("Error during ping execution: {}", e);
            });
            //Elasped time calculation
            let elapsed = start.elapsed();

            let mut time: u16 = 0;
            //Convert elasped time in ms to display after the ping latency
            time = elapsed.as_millis().try_into().unwrap();

            //Disable raw terminal to prevent displaying issue of println!
            use crossterm::terminal::disable_raw_mode;
            disable_raw_mode()?;
            println!("ping n°{} ping latency: {} ms \n", i, time);
            enable_raw_mode()?;

            //If the ping is >= to 5 sec it is a timeout
            if time >= 5000{
            self.data.push(5000);
            self.sys_time.push(self.get_time());

            }
            else{
            self.data.push(time);
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
            
            //Do a sleep of 500ms to do ping each 500 ms => Around 2 pings/sec depending of the ping latency of course
            sleep(Duration::from_millis(500)).await;
            }
            
        //Disable raw terminal to prevent displaying issue of println!    
        use crossterm::terminal::disable_raw_mode;
        disable_raw_mode()?;

        //Call this method to calculate and display ping results stats when the number of ping exceed the number defined by the user
        self.latency_data();
    Ok(())
    }

    
}

//Specific methods of PingTool that not match with the NetworkTool Trait general definition
impl PingTool{
    //Function to set PingTool attributes
    #[allow(dead_code)]
    fn setting (&mut self, output: &str, nb_ping: u16){
        self.output = output.to_string();
        self.nb_ping = nb_ping;

    }

    //Init attributes of the object
    pub fn new(target: &str, output: &str, nb_ping: u16) -> Self {
        Self {
            target: target.to_string(),
            output: output.to_string(),
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
        }
    }

}