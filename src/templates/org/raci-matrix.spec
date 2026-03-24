## Config
title: RACI Matrix
flow = TB

## Nodes
- [feature] Feature Launch {bold}
- [pm] Product Manager {person}
- [eng] Engineering Lead {person}
- [design] Designer {person}
- [legal] Legal {person}
- [r_pm] Responsible {fill:#4caf50}
- [a_pm] Accountable {fill:#2196f3}
- [c_eng] Consulted {fill:#ff9800}
- [i_legal] Informed {fill:#9e9e9e}

## Flow
feature --> r_pm: PM Responsible
feature --> a_pm: PM Accountable
feature --> c_eng: Eng Consulted
feature --> i_legal: Legal Informed
