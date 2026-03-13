# E-Commerce Database Schema

Entity-relationship diagram for a simple e-commerce platform.

## Config
bg = dots
flow = LR

## Nodes
- [users] users {entity} {highlight}
  id (uuid) [PK]
  email (varchar)
  name (varchar)
  created_at (timestamp)

- [orders] orders {entity} {highlight} {note:Core transaction table}
  id (uuid) [PK]
  user_id (uuid) [FK]
  status (varchar)
  total (decimal)
  created_at (timestamp)

- [products] products {entity}
  id (uuid) [PK]
  name (varchar)
  price (decimal)
  stock (int)
  category_id (uuid) [FK]

- [order_items] order_items {entity}
  id (uuid) [PK]
  order_id (uuid) [FK]
  product_id (uuid) [FK]
  quantity (int)
  unit_price (decimal)

- [categories] categories {entity} {note:Supports nested categories}
  id (uuid) [PK]
  name (varchar)
  parent_id (uuid) [FK]

## Flow
users "places" --> orders {c-src:1} {c-tgt:0..N}
orders "contains" --> order_items {c-src:1} {c-tgt:1..N}
products "in" --> order_items {c-src:1} {c-tgt:0..N}
categories "groups" --> products {c-src:1} {c-tgt:0..N}
categories "parent" --> categories {c-src:0..1} {c-tgt:0..N} {dashed}
