## Config
title: Org Chart
flow = TB

## OrgTree
- [ceo] CEO {hexagon} {fill:#f9e2af} {bold}
  - [cto] CTO {rounded} {fill:#89b4fa} {bold}
    - [eng_lead] Eng Lead {person} {fill:#89b4fa} {ok}
      - [backend] Backend Team {rounded} {fill:#74c7ec} {ok}
      - [frontend] Frontend Team {rounded} {fill:#74c7ec} {ok}
  - [cpo] CPO {rounded} {fill:#cba6f7} {bold}
    - [pm] PM {person} {fill:#cba6f7} {ok}
    - [ux] UX {person} {fill:#cba6f7} {wip}
  - [cfo] CFO {rounded} {fill:#f2cdcd} {bold}
    - [finance] Finance Team {rounded} {fill:#f2cdcd} {info}
