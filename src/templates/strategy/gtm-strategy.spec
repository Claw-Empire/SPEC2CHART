## Config
title: GTM Strategy
flow = TB

## Nodes
- [hn] Launch Post {rounded} {fill:#4a90d9}
- [blog] Dev Blog {rounded} {fill:#4a90d9}
- [dl] Free Download {hexagon}
- [gh] GitHub Stars {hexagon}
- [pro] Pro Plan {diamond} {fill:#4caf50}

## Flow
hn --> dl: traffic
blog --> gh: stars
dl --> pro: upgrade path
gh --> pro: conversion
dl --> gh: cross-channel
