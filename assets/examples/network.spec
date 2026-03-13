# Office Network Topology

Network diagram showing a typical office network infrastructure with segmented VLANs.

## Config
bg = dots
flow = TB

## Nodes
- [internet] Internet {internet} {sublabel:ISP · 1Gbps}
  External internet connection via ISP uplink.
- [fw] Firewall {diamond} {fill:red} {critical} {highlight}
  Perimeter security device. All traffic filtered here.
- [router] Core Router {hexagon} {fill:blue} {sublabel:Layer 3}
  Layer 3 routing between network segments.
- [sw_core] Core Switch {connector} {sublabel:10G uplinks} {highlight}
  High-speed backbone switching.
- [sw_user] User Switch {sublabel:1G ports}
  Access layer for workstations.
- [sw_server] Server Switch {sublabel:10G dedicated}
  Dedicated server segment.
- [ap1] WiFi AP 1 {circle} {sublabel:Floor 1}
  2.4/5GHz wireless access point, floor 1.
- [ap2] WiFi AP 2 {circle} {sublabel:Floor 2}
  2.4/5GHz wireless access point, floor 2.
- [file] File Server {fill:teal} {note:Shared storage + backups}
  Shared storage and backups.
- [db] Database Server {circle} {fill:blue} {sublabel:PostgreSQL}
  Production database cluster.
- [web] Web Server {fill:teal} {sublabel:Intranet}
  Internal intranet and web services.
- [pc1] Workstations {sublabel:DHCP 192.168.1.x}
  User endpoints.
- [vpn] VPN Gateway {hexagon} {fill:purple} {note:Remote staff access}
  Remote access for staff.

## Notes
- Segmented into DMZ, server, and user VLANs {yellow}
- All servers use static IP allocation {ok}
- WiFi uses WPA3 enterprise with RADIUS {info}

## Flow
internet -> fw {thick}
fw -> router {thick}
fw -> vpn {dashed}
router -> sw_core {thick}
sw_core -> sw_user
sw_core -> sw_server
sw_user -> ap1
sw_user -> ap2
sw_user -> pc1
sw_server -> file
sw_server -> db
sw_server -> web
vpn -> sw_user {dashed} {note:tunnel}
