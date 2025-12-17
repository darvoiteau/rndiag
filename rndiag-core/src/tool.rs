use anyhow::Result;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use dns_lookup::{lookup_host};
use std::fs::File;
use std::io::{self};
use csv::Writer;

   //NetworkTool trait => The trait for all networktool (ping, DNS resolving, telnet connection, ...)
   #[allow(async_fn_in_trait)]
    pub trait LatencyTool {
        fn name(&self) -> &'static str;
        async fn run(&mut self) -> Result <(), io::Error>;
        fn data(&self) -> &Vec<u16>; //Get the var content of the current object in the trait
        fn nb_ping(&self) -> &u16; //Get the var content of the current object in the trait
        fn sys_time(&self) -> &Vec<u64>; //Get the var content of the current object in the trait
        fn begin_time(&self) -> &u64; //Get the var content of the current object in the trait
        fn elapsed_time(&mut self) -> &mut u64; //Get the var content of the current object in the trait
        fn latency_time(&mut self) -> &mut Vec<u64>; //Get the var content of the current object in the trait
        fn latency_min(&mut self) -> &mut Vec<u16>; //Get the var content of the current object in the trait
        fn latency_moy(&mut self) -> &mut Vec<u16>; //Get the var content of the current object in the trait
        fn latency_max(&mut self) -> &mut Vec<u16>; //Get the var content of the current object in the trait
        fn latency_min_sampled(&mut self) -> &mut Vec<u64>; //Get the var content of the current object in the trait
        fn latency_moy_sampled(&mut self) -> &mut Vec<u64>; //Get the var content of the current object in the trait
        fn latency_max_sampled(&mut self) -> &mut Vec<u64>; //Get the var content of the current object in the trait
        fn output(&self) -> &str;
        fn target(&self) -> &str;
        
        //Function to calculate stats about the ping, dns resolver, ... => min/avg/max + % of packet loss when we quit the program
        fn latency_data(&mut self){

            //Get the vecsize to know the number of ping and number of related lantencies values
            let vec_size = self.sys_time().len();


                //Calculate the min and max latency on all executed ping
                let max_latency = *self.data().iter().max().unwrap();
                let min_latency = *self.data().iter().min().unwrap();

                //Get the sum to calcul the moy latency on all executed ping
                let sum_latency: u16 = self.data().iter().sum();
                let moy_latency = sum_latency as u16 / vec_size as u16;

                //Calculate the number of no received packets
                //5000ms for a packet is considered as timeout and a no received packet
                let zero = self.data().iter().filter(|&&x| x == 5000).count() as u16;
                let received = vec_size as u16 - zero;

                
                //Calculate the % of packet loss
                let packet_loss_percent: f64 = ((vec_size as f64 - received as f64) / vec_size as f64) * 100.0;

                //Display results
                println!("--- Statistics ---\n");
                println!("{} packet transmitted, {} packet received, {:.2}% packet loss\n", vec_size, received, packet_loss_percent);
                println!("round-trip min/avg/max = {}/{}/{} ms\n", min_latency, moy_latency, max_latency);

            

        

    }

    //Function for sampling data depending of the elapsed_time and the scale sampling
    fn sampling(&mut self, mut j: usize, scale: u16) -> usize{

        //Get the number of ping by the size of sys_time. To remind, each ping have a timestamps related value. So sys_time number values ping number values
        let mut vec_size = self.sys_time().len();

        //used to determine the number of ping that are in the interval time defined by the sampling scale
        let mut i = 0;

        //To have the number of elements of Max,moy,min after the for boucle
        let mut l: usize = 0;

        //Index used to put the system timestamp each time we do the pre-sampling
        let mut n: usize = 0;

        //Get and update the elapsed_time
        *self.elapsed_time() = self.sys_time()[vec_size - 1] - self.sys_time()[0];


        //Create temporary vec for pre-sampling
        let mut vec_max_temp= Vec::new();
        let mut vec_moy_temp= Vec::new();
        let mut vec_min_temp= Vec::new();
        let mut vec_time_temp: Vec<u64> = Vec::new();

        //Index used for the pre-sampling. Pre-sampling => max,min,moy,... latency calculation for each 5 pings. k is the index to determine when we have 5 ping and do pre-sampling
        let mut k = 0;

        //Temporary vec that contain pings latency values result for pre-sampling
        let mut vec_data_temp: Vec<u16> = Vec::new();

        //Each time we change the scale, the scale value = 0 and fix the vec_size to 0 to pre-sampling again all values for the new scale.
        if scale == 0{
         vec_size = 0;
        }

        //Loop for pre-sampling data. Pre-sampling => max,min,moy,... latency calculation for each 5 pings
        for elem in &self.data()[vec_size-scale as usize..]{
            
            //Push elem for pre-sampling while k <= 4 in vec_data_temp 
            if k <= 4{
               vec_data_temp.push(*elem);
               k +=1;
            }
            //When k == 4 its mean we have 5 pings and need to pre-sample it.
            if k == 4{

                //Get max,moy,min latency for each batch of 5 pings latency result => The pre-sampling
                let max_latency: u16 = *vec_data_temp.iter().max().unwrap();
                let min_latency = *vec_data_temp.iter().min().unwrap();
                let sum_latency: u16 = vec_data_temp.iter().sum();
                let moy_latency: u16 = sum_latency as u16 / k as u16;

                //Push pre-sampled (so max,moy,min latency value of 5 pings) values in temporary vecs
                vec_max_temp.push(max_latency);
                vec_min_temp.push(min_latency);
                vec_moy_temp.push(moy_latency);
                vec_time_temp.push(self.sys_time()[n]);
                //Clear vec for push next 5 pings latencies values for next loop trips and next pre-sampling calculation.
                vec_data_temp.clear();
                 
                k = 0;

                //Increase the l value to know the number of elem in temporary max/moy/min vec
                l+=1;
            }
            n+=1;
        }
        //Index of below while loop  
        let mut m: usize = 0;

        //Push stored pre-sampled values in temporary vectors to current object attribute vec
        while m < l{
            self.latency_max().push(vec_max_temp[m]);
            self.latency_min().push(vec_min_temp[m]);
            self.latency_moy().push(vec_moy_temp[m]);
            self.latency_time().push(vec_time_temp[m]);
            m +=1;

        }

        //Clear pre-sampled vec to free memory
        vec_max_temp.clear();
        vec_moy_temp.clear();
        vec_min_temp.clear();
        vec_time_temp.clear();

        

        //Sampling part => We sampling pre-sampled latencies values
        //Depending of elapsed time (5min, 1h, 2h, ...) we pass in a condition or in other one
        if *self.elapsed_time() <= 300{
            //Main loop for sampling
            while i < l{
                //The starting timestamp ping for the sampling window.
                //Take the j value in latency-time +5 => the scale value.
                let start_sampling_time = self.latency_time()[j] + 5;

                //Boucle for to determine with i index the number of pings that can be in the window of the current scale for sampling with timestamp
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 5 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     //Exceed the timestap in the window of sampling => exit the loop
                     else{
                        break;
                     }
                     

                }
                
                //Create interval with i and j to do a moy with the correct value
                let interval = i-j;

                //j is the begin of the window sampling and i the end of this window so we create an interval of data latencies ping to take for sampling with j and i
                //Do the sampling, also interval between i and j = scale value. 5 pings for 5 min scale, 15 pings for 30 min scale, 30 pings for 1h scale, ...
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                //Push sampled data in the current object attributes.
                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);

                //For the next sampling, the window of sampling will change, so the end of the current window is the begining of the next window
                j = i;
                

            } 



        }

        else if *self.elapsed_time() <= 1800{
            while i < l{
                let start_sampling_time = self.latency_time()[j] + 15;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 15 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;
                
                //When the scale just change we resample again all current data with the new scale of sampling => Avoid loosing data for graph
                if j == 0{
                  //Take number of latencies values to resample it entirely with the new scale of sampling
                  let len = self.data().len();
                  //Index used to browse all current data (ping latencies values)
                  let mut o: usize = 0;
                  
                  //Loop to resampling all current data
                  // len - 1 to not exceed the index in self.data()
                  while o < len - 1 {

                     //Index used to push a specific amount of data according to the value of scale
                     let mut n: usize = 0;

                     //Temporary vec for sampling
                     let mut elem_temp: Vec<u16> = Vec::new();
                        //Loop to push data in elem_temp while n < to the value scale for a sampling after
                        while n < 15 {
                           elem_temp.push(self.data()[o]);
                           n +=1;

                           //Detect if we are at the end of the vec that contain all ping datas. if it is the case we break this loop and because the main while is o < len -1 we quit also the main while loop
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     //Sampling all ping values scale by scale (elem numbers elem_temp = scale value)
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 15 as u16;

                     //Push just sampled all curent ping value to the current object attribute
                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }
               //If we not change the scale of sampling => Normal sampling
               else{
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
                
               }
               j = i;

            } 
        }

        else if *self.elapsed_time() <= 3600 {
            while i < l{
                let start_sampling_time = self.latency_time()[j] + 30;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 30 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 30 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 30 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
               

            } 
        }
        else if *self.elapsed_time() <= 7200{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 60;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 60 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 60 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 60 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 



        }
        else if *self.elapsed_time() <= 14400{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 120;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 120 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 120 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 120 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }
               else {

                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 

        }
        else if *self.elapsed_time() <= 28800{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 240;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <=240 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 240 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 240 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 
            
        }
        else if *self.elapsed_time() <= 57600{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 480;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 480 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 480 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 480 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {

                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 
            
        }
        else if *self.elapsed_time() <= 115200{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 960;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 960 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 960 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 960 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 

        }
        else if *self.elapsed_time() <= 230400{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 1920;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 1920 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 1920 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 1920 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
                j = i;
                

            } 

        }
        else if *self.elapsed_time() <= 460800{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 3840;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 3840 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;
                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 3840 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 3840 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 
        }
        else if *self.elapsed_time() <= 921600{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 7680;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 7680 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 7680 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 7680 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 

        }
        else if *self.elapsed_time() <= 1843200{
             while i < l{
                let start_sampling_time = self.latency_time()[j] + 15360;
                for elem in self.latency_time(){
                     if start_sampling_time as i64 - *elem as i64 <= 15360 || start_sampling_time as i64 - *elem as i64 == 0 {
                        i+=1;
                        
                     }
                     else{
                        break;
                     }
                     

                }
                
                let interval = i-j;

                if j == 0{
                  let len = self.data().len();
                  let mut o: usize = 0;
                  while o < len - 1 {

                  
                     let mut n: usize = 0;
                     let mut elem_temp: Vec<u16> = Vec::new();
                        while n < 15360 {
                           elem_temp.push(self.data()[o]);
                           n +=1;
                           if o < len -1 {
                              o +=1;
                           }
                           else{
                              break;
                           }

                        }
                     
                     let max_latency_sample = elem_temp.iter().max().unwrap();
                     let min_latency_sample = elem_temp.iter().min().unwrap();
                     let sum_latency_sample: u16 = elem_temp.iter().sum();
                     let moy_latency_sample = sum_latency_sample as u16 / 15360 as u16;

                     self.latency_max_sampled().push(*max_latency_sample as u64);
                     self.latency_moy_sampled().push(moy_latency_sample as u64);
                     self.latency_min_sampled().push(*min_latency_sample as u64);

                  }
               }

               else {
                let max_latency_sample = *self.latency_max()[j..i].iter().max().unwrap();
                let min_latency_sample = *self.latency_min()[j..i].iter().min().unwrap();
                let sum_latency_sample: u16 = self.latency_moy()[j..i].iter().sum();
                let moy_latency_sample = sum_latency_sample as u16 / interval as u16;

                self.latency_max_sampled().push(max_latency_sample as u64);
                self.latency_moy_sampled().push(moy_latency_sample as u64);
                self.latency_min_sampled().push(min_latency_sample as u64);
               }
                j = i;
                

            } 
        }

        j

    }

    //Export result in CSV
    fn export_csv(&mut self) -> Result <(), io::Error>{
        //Create the output csv file
        let file_export = File::create(&self.output());
        let mut i: usize = 0;

        //Take the error if we cannot create the output csv file
        let file = match file_export{
            Ok(f) => f,
            Err(e) => {
                panic!("Creation file error !: {}", e);
            }
        };

        //Create a writer
        let mut writer = Writer::from_writer(file);

        //Write columns title in the output csv file
        if let Err(e) = writer.write_record(&["Date", "Ping Number", "Latency"]) {
        eprintln!("Error! : {}", e);
        }

        //Get the number of ping values for writing
        let len = self.sys_time().len();
        
        //Write ping latencies values in the csv output file.
        while i < len{
            let data = self.data()[i];
            let j = i;
            let sys_time = self.sys_time()[i];
            if let Err(e) = writer.write_record(&[sys_time.to_string(), j.to_string(), data.to_string()]) {
                eprintln!("Error writing to CSV! {}", e);
            }
            i+=1;
        }
        // Flush the writer to ensure all data is written
        writer.flush().expect("Failed to flush writer");
        Ok(())
    }

    //Little function to get the current system time timestamp
    fn get_time(&self) -> u64 {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as u64;
        timestamp
    }

    //resolve method
    //Resolve hostname to get IP when the user give hostname instead of IP
    fn resolve(&mut self) -> IpAddr{
      let result_ip = lookup_host(&self.target()).unwrap().collect::<Vec<_>>();
      result_ip[0]
    }

}

#[allow(async_fn_in_trait)]
//ConnecTool Definition
pub trait ConnectTool{
   fn name(&self) -> &'static str; //Get the name of the object
   fn srv_addr(&self) -> &str; //Get srv_addr object attribute
   async fn run(&mut self) -> std::io::Result<()>; //Run method object
   async fn start_server(&mut self) -> std::io::Result<()>; //Start_server method => Launch server part and handle connections
   async fn client(&mut self) -> std::io::Result<()>; //client method => Launch client part

   //resolve method
   //Resolve hostname to get IP when the user give hostname instead of IP
   fn resolve(&mut self) -> IpAddr{
      let result_ip = lookup_host(&self.srv_addr()).unwrap().collect::<Vec<_>>();
      result_ip[0]
    }
}