# Rndiag Changelog

## Version 1.1.2
- Fix issue #1 "Terminal crash". Linux terminal crashed when you launched rndiag with the tcp ping in IPv6 without sudo

- Rndiag crashed when you tried to show the graph before to have 5 ping
  - Now, rndiag do not crash and the graph display a message to inform the lack of ping to display data

- Fix displayed average data in graph when you launch rndiag for couple of hours/days
  - This bug was introduced in the previous release and the calcul of the average before to display it in graph was wrong

- Rndiag crashed over 9h of ping. This release fix this issue

- Improvement of errors handling and management
  - Since release 1.1.1 a work is in progress to handle and manage correctly errors
  - Some errors was not handled correctly
  - Some errors broken the Linux terminal or broken the Linux terminal displaying
  - Some important errors was handled but didn't stopped rndiag
  - ...

- Add 

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
