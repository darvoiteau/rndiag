//Count the number of received packet from the lasts pings tests during the diagnostic
pub fn packet_received(packet: &Vec<u16>) -> u8{

    let zero: u8 = packet.iter().filter(|&&x| x == 5000).count() as u8;
    zero
}

//Check the latency of lasts pings tests and set a level, 0 => OK, 1 => Warning, 2 => Critical
pub fn packet_latency(latency_moy_sampled: &u64) -> u8{
    let latency1: u64 = 59;
    let latency2: u64 = 130;

        if latency_moy_sampled >= &latency2{
            2

        }
        else if latency_moy_sampled >= &latency1 {
            1
        }
        else {
            0
        }

}

//Format the output cli diagnostics depending of latencies and recived packets
pub fn output_format(latency_moy_vec: &Vec<u8>, zero_vec: &Vec<u8>){
    println!("\n=========== Diagnostic Result ===========\n");
    
    println!("-----Ping Result:");
    if zero_vec[0] > 0 && zero_vec[0] < 5 {
        println!("Connectivity {}/6 failed ⚠", zero_vec[0]);
    }
    else if zero_vec[0] >= 5 {
        println!("Connectivity ❌");
    }
    else {
        println!("Connectivity ✅");
    }

    if latency_moy_vec[0] == 1 {
        println!("Latency ⚠");
    }
    else if latency_moy_vec[0] == 2 {
        println!("Latency ❌");
    }
    else {
        println!("Latency ✅");
    }

    println!("\n-----TCP Ping Result:");
    if zero_vec[1] > 0 && zero_vec[1] < 5 {
        println!("Connectivity {}/6 failed ⚠", zero_vec[1]);
    }
    else if zero_vec[1] >= 4 {
        println!("Connectivity ❌");
    }
    else {
        println!("Connectivity ✅");
    }

    if latency_moy_vec[1] == 1 {
        println!("Latency ⚠");
    }
    else if latency_moy_vec[1] == 2 {
        println!("Latency ❌");
    }
    else {
        println!("Latency ✅");
    }


    println!("\n-----Name Resolver Result:");
    if zero_vec[2] > 0 && zero_vec[2] < 5 {
        println!("Connectivity {}/6 failed ⚠", zero_vec[2]);
    }
    else if zero_vec[2] >= 5 {
        println!("Connectivity ❌");
    }
    else {
        println!("Connectivity ✅");
    }

    if latency_moy_vec[2] == 1 {
        println!("Latency ⚠");
    }
    else if latency_moy_vec[2] == 2 {
        println!("Latency ❌");
    }
    else {
        println!("Latency ✅");
    }



}
