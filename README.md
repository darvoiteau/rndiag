# Rndiag

A program developed in rust to check and diagnostic network latencies and connecivites issues.
Rndiag is designed to be a toolkit for diagnosing and testing these network issues.
Rndiag is also designed to follow network latency drift over time

## Features
- Ping tool
- Tcp ping tool : send tcp packets with specific TCP Flag on specific port like ping to see latency and the server responses
- Resolver tool : Resolve specified hostname/IP like a ping to see resolution latencies
- Speedtest: Client/Server mode to test the bandwidth
- TCP message: Simple Client/Server message server like netcat to check the connectivity between 2 host
- Graph: To see Ping tool, Tcp ping tool, Resolver tool ping latencies result in graphs to a better view in the time
- Prometheus exporter: Rndiag can be launched as exporter to collect latencies metrics
- Diagnostic: Quick network diagnostic to help determine network issues

## Installation
Download the binary that match your cpu arch in Releases section of this repo
Rename the binary by the following name: "rndiag"
And to install it on your system do following commands:
```bash
chmod +x rndiag
mv rndiag /usr/bin
```

## Usage
```bash
Usage: rndiag-cli [-d <dst>] [-c <count>] [-o <output>] [-p <port>] [-m <mode>] [-s <server>] [-t <time>] [-b <bitrate>] [-f <flag>] [-D <diagnostic>] [-P <ping>] [-T <tping>] [-R <resolver>] [-S <sptest>] [-N <nc>] [--exporter <exporter>] [--ws-addr <ws-addr>] [--ws-port <ws-port>]

reach new args

Options:
  -d, --dst         destination server ip or name
  -c, --count       stop after <count> replies
  -o, --output      output csv filename
  -p, --port        destination port
  -m, --mode        mode for speedtest, upload => upload, download => download,
                    full => upload + download
  -s, --server      for tools in server-client mode, true => run as server,
                    false => run as client, default => false
  -t, --time        speedtest duration in secs.
  -b, --bitrate     target bitrate in Mbps, default 0 for unlimited
  -f, --flag        tcp flag for tcp_ping. S => SYN, A => ACK, R => RST, F =>
                    FIN, P => PUSH, U => URG
  -D, --diagnostic  quick network diagnostics (ping latency, resolution latency,
                    tcp_ping). to use diagnostic -D => TrueUsage: rndiag D
                    <speedtestSrv> -d => specify specific server to resolve and
                    to contact for ping and tcp_ ping, -p => specify specific
                    port to contact for tcp_ping
  -P, --ping        to use ping, -P => true + specify destination -d
  -T, --tping       to use tcp ping, -T => true + specify destination -d and
                    port -p
  -R, --resolver    to use DN resolver, -R => true + specify the server to
                    resolve -d
  -S, --sptest      client-server tool,to launch it on client side, -S + specify
                    the server -d + specify the port -p + specify mode -m. on
                    server side -S + -s true + specify the listening addr -d +
                    the listening port -p
  -N, --nc          client-server tool, to launch it on client side, -S +
                    specify the server -d + specify the port -p.  on server side
                    -S + -s true + specify the listening addr -d + the listening
                    port -p
  --exporter        provide a web-page with metrics tht can be scrapped by
                    prometheus/grafana, --exporter true
  --ws-addr         IP of the web-server for exporter mode
  --ws-port         port of the web-server for exporter mode
  --help, help      display usage information

```

### Launch graph
If you use Ping tool, TCP Ping tool, Resolver, during the execution and at any time press 'g' key and the graph will be displayed.
During the graph displaying the execution of ping is in pause


## Example Usage

### Launch basic ping

sudo rndiag -P true -d <IP/host>
```bash
sudo rndiag -P true -d 8.8.8.8

ping n°0 ping latency: 16 ms 

ping n°1 ping latency: 13 ms 

ping n°2 ping latency: 13 ms 

ping n°3 ping latency: 13 ms 

ping n°4 ping latency: 12 ms 

ping n°5 ping latency: 21 ms 

ping n°6 ping latency: 16 ms 

--- Statistics ---

7 packet transmitted, 7 packet received, 0.00% packet loss

round-trip min/avg/max = 12/14/21 ms

```

### Launch tcp ping with a SYN flag
sudo rndiag -T true -d <IP/Host> -p <port> -f S

```bash
sudo rndiag -T true -d google.com -p 443 -f S

TCP-PING google.com:443 from 192.168.50.8:54321 flags=0x02 (SYN) count=0
[1] Reply in 10.212 ms - flags=0x12 (SYN|ACK)
[2] Reply in 12.133 ms - flags=0x12 (SYN|ACK)
[3] Reply in 9.193 ms - flags=0x12 (SYN|ACK)
[4] Reply in 11.766 ms - flags=0x12 (SYN|ACK)
[5] Reply in 15.494 ms - flags=0x12 (SYN|ACK)
[6] Reply in 12.048 ms - flags=0x12 (SYN|ACK)
[7] Reply in 12.401 ms - flags=0x12 (SYN|ACK)
--- Statistics ---

7 packet transmitted, 7 packet received, 0.00% packet loss

round-trip min/avg/max = 9/11/15 ms
```

### Launch speedtest
On server side: rndiag -S true -d 192.168.1.50 -p 8080 -s true
Speedtest is a server-client mode, so -s true => run rndiag as server

On client side: rndiag -S true -d 192.168.1.110 -p 8080 -s false
-s false => run rndiag as client

### Launch tcp message (netcat like)
On server side: 
```bash
rndiag -N true -d 192.168.1.50 -p 8080 -s true
```
Speedtest is a server-client mode, so -s true => run rndiag as server

On client side: 
```bash
rndiag -N true -d 192.168.1.110 -p 8080 -s false
```
-s false => run rndiag as client

### Launch rndiag as exporter
```bash
sudo ./rndiag-cli --exporter true -d google.fr -p 443 --ws-addr 192.168.1.149 --ws-port 8080 -o metrics.txt
```
It will launch all latency tools (Ping, TCP Ping, Resolver) on the defined target (-d) and defined port for TCP Ping (-p)
--ws-addr => it is the listening addr for the rndiag web-server that expose metrics
--ws-port => it is the listening port for the rndiag web-server that expose metrics
-o => the file that will contain metrics and exposed by the web-server

### Launch diagnostic
sudo rndiag -D true

```bash
sudo rndiag -D true


ping n°0 ping latency: 13 ms 

ping n°1 ping latency: 12 ms 

ping n°2 ping latency: 12 ms 

ping n°3 ping latency: 12 ms 

ping n°4 ping latency: 12 ms 

ping n°5 ping latency: 11 ms 

--- Statistics ---

6 packet transmitted, 6 packet received, 0.00% packet loss

round-trip min/avg/max = 11/12/13 ms

TCP-PING 1.1.1.1:443 from 192.168.50.8:54321 flags=0x02 (SYN) count=6
[1] Reply in 11.885 ms - flags=0x12 (SYN|ACK)
[2] Reply in 11.504 ms - flags=0x12 (SYN|ACK)
[3] Reply in 12.198 ms - flags=0x12 (SYN|ACK)
[4] Reply in 11.575 ms - flags=0x12 (SYN|ACK)
[5] Reply in 9.608 ms - flags=0x12 (SYN|ACK)
[6] Reply in 9.480 ms - flags=0x12 (SYN|ACK)
--- Statistics ---

6 packet transmitted, 6 packet received, 0.00% packet loss

round-trip min/avg/max = 9/10/12 ms

DNS request n°0 DNS request latency: 15 ms
DNS query result: one.one.one.one

DNS request n°1 DNS request latency: 2 ms
DNS query result: one.one.one.one

DNS request n°2 DNS request latency: 2 ms
DNS query result: one.one.one.one

DNS request n°3 DNS request latency: 2 ms
DNS query result: one.one.one.one

DNS request n°4 DNS request latency: 3 ms
DNS query result: one.one.one.one

DNS request n°5 DNS request latency: 2 ms
DNS query result: one.one.one.one

--- Statistics ---

6 packet transmitted, 6 packet received, 0.00% packet loss

round-trip min/avg/max = 2/4/15 ms


=========== Diagnostic Result ===========

-----Ping Result:
Connectivity ✅
Latency ✅

-----TCP Ping Result:
Connectivity ✅
Latency ✅

-----Name Resolver Result:
Connectivity ✅
Latency ✅

```