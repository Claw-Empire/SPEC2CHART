## Config
title: RACI Matrix
flow = TB

## Nodes
- [feature] Feature Launch {diamond} {fill:#f9e2af} {wip}
  Cross-functional delivery.
- [pm] Product Manager {person} {fill:#89b4fa} {ok}
  Responsible — drives.
- [eng] Engineering Lead {person} {fill:#cba6f7} {ok}
  Accountable — ships.
- [design] Designer {person} {fill:#a6e3a1} {info}
  Consulted — UX.
- [legal] Legal {person} {fill:#f2cdcd} {info}
  Consulted — compliance.
- [support] Support {person} {fill:#b4befe} {todo}
  Informed — docs.
- [launch] Launch Complete {rounded} {fill:#a6e3a1} {done}
  Shipped to prod.

## Flow
feature --> pm: Responsible
feature --> eng: Accountable
feature --> design: Consulted
feature --> legal: Consulted
feature --> support: Informed
pm --> launch: drives delivery
eng --> launch: reviews & approves
design --> launch: signs off UX
