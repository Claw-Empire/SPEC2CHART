## Config
flow = TB

## Swimlane: Awareness
- [hn] Launch Post {star} {done}
- [blog] Dev Blog {star} {wip}

## Swimlane: Acquisition
- [dl] Free Download {hexagon} {metric:500 users}
- [gh] GitHub Stars {hexagon} {metric:1.2k}

## Swimlane: Revenue
- [pro] Pro Plan {diamond} {metric:$12/mo} {todo}

## Flow
hn --> dl: traffic
blog --> gh: stars
dl --> pro: upgrade path
