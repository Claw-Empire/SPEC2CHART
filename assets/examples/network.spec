# Office Network Topology

Network diagram showing a typical office network infrastructure.

## Nodes
- [internet] Internet {parallelogram} {fill:yellow}
  External internet connection.
- [fw] Firewall {diamond} {fill:red} {critical}
  Perimeter security device. All traffic filtered here.
- [router] Core Router {hexagon} {fill:blue}
  Layer 3 routing between network segments.
- [sw_core] Core Switch {connector}
  High-speed backbone switching (10G uplinks).
- [sw_user] User Switch
  Access layer for workstations (1G ports).
- [sw_server] Server Switch
  Dedicated server segment (10G).
- [ap1] WiFi AP 1 {circle}
  2.4/5GHz wireless access point, floor 1.
- [ap2] WiFi AP 2 {circle}
  2.4/5GHz wireless access point, floor 2.
- [file] File Server {fill:teal}
  Shared storage and backups.
- [db] Database Server {circle} {fill:blue}
  Production database cluster.
- [web] Web Server {fill:teal}
  Internal intranet and web services.
- [pc1] Workstations
  User endpoints (DHCP 192.168.1.0/24).
- [vpn] VPN Gateway {hexagon} {fill:purple}
  Remote access for staff.

## Notes
- Segmented into DMZ, server, and user VLANs {yellow}
- All servers use static IP allocation {ok}
- WiFi uses WPA3 enterprise with RADIUS {info}

## Flow
internet -> fw {thick}
fw -> router {thick}
fw -> vpn {dashed}
router -> sw_core {thick} {glow}
sw_core -> sw_user
sw_core -> sw_server
sw_user -> ap1
sw_user -> ap2
sw_user -> pc1
sw_server -> file
sw_server -> db
sw_server -> web
vpn -> sw_user {dashed}
