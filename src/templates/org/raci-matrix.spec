## Config
title: RACI Matrix
flow = TB

## Nodes
- [feature] Feature Launch {diamond}
- [pm] Product Manager {person}
- [eng] Engineering Lead {person}
- [design] Designer {person}
- [legal] Legal {person}
- [launch] Launch Complete {rounded} {fill:#4caf50}

## Flow
feature --> pm: Responsible
feature --> eng: Consulted
feature --> design: Consulted
feature --> legal: Informed
pm --> launch: drives delivery
eng --> launch: reviews & approves
pm --> eng: Accountable to
