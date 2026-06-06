# Rndiag Changelog

## Version 1.1.2
- Fix issue #1 "Terminal crash". Linux terminal crashed when you launched rndiag with the tcp ping in IPv6 without sudo

- Rndiag crashed when you tried to show the graph before having 5 ping
  - Now, rndiag do not crash and the graph displays a message to inform of the lack of pings to display data

- Fix displayed average data in graph when you launch rndiag for a couple of hours/days
  - This bug was introduced in the previous release and the calculation of the average before displaying it in the graph was wrong

- Rndiag crashed after 9h of ping. This release fixes this issue

- Improvement of error handling and management
  - Since release 1.1.1, work has been in progress to handle and manage errors correctly
  - Some errors were not handled correctly
  - Some errors broke the Linux terminal or broke the Linux terminal display
  - Some important errors were handled but didn't stop rndiag
  - ...

- Add a Rndiag executable binary for armv7

## Version 1.1.1
- Fix exporter exposed metrics format => some syntax issues fixed, the destination is now correctly displayed in target="XXX"

- IPv6 bugfix on TCP_Ping, in the previous release it was not supported => Now tcp_ping works in IPv6, rndiag is now fully IPv6 compatible

- Fix average result on all latency tools. When you exited rndiag after 1h, the displayed average in results was always O.

- Improve error management and handling

- Various improvements

## Version 1.1.0
- Now, by default output is disabled. To save output in file please use explicitly '-o' option

- IPv6 is now fully compatible with rndiag. In the previous version some tools was not compatible

- Fix low bandwidth issue, when an upload speedtest was performed.

- Add x86_64 musl (embedded libraries) compiled binary for better compatibility with different libc version on different linux

- Fix some typos in the --help menu

## Version 1.0.0
- Initial commit
