# VC Fund Architecture

A venture capital fund structure showing the complete lifecycle of capital —
from LP commitment through investment deployment, portfolio management,
exit events, and returns distribution. Typical fund size: $50M-$500M
with a 10-year term (5 years investment period + 5 years harvest).

## Nodes

- [lps] Limited Partners (LPs)
  Institutional investors (pension funds, endowments, family offices)
  and high-net-worth individuals who commit capital to the fund.
  Typical minimum commitment: $1M-$25M per LP.

- [fund] VC Fund
  The pooled investment vehicle, structured as a limited partnership.
  Capital is drawn down over 3-5 years via capital calls as deals close.

- [gp] General Partner (GP)
  The fund manager responsible for sourcing deals, conducting due
  diligence, making investment decisions, and managing the portfolio.

- [mgmt] Management Company
  The GP's operating entity that employs the team, covers overhead,
  and charges the fund a 2% annual management fee on committed capital.

- [deal] Deal Flow & Due Diligence {diamond}
  The screening funnel: ~1000 companies reviewed, ~100 meetings,
  ~10 deep dives, ~3-5 investments per year.

- [co_a] Portfolio Co A
  Early-stage SaaS company. Series A investment of $5M for 20% ownership.

- [co_b] Portfolio Co B
  Biotech startup. Seed investment of $2M for 15% ownership.

- [co_c] Portfolio Co C
  Fintech platform. Series B co-investment of $8M for 12% ownership.

- [exit] Exit Events {diamond}
  IPO, M&A acquisition, or secondary sale. Typical hold period: 5-7 years.
  Target return: 3x-10x per successful exit.

- [returns] Returns Distribution
  Proceeds are distributed according to the waterfall: first return
  of capital to LPs, then preferred return (8% hurdle), then splits.

- [carry] Carried Interest (20%)
  The GP's performance fee — typically 20% of profits above the
  preferred return hurdle. This is the primary GP compensation.

## Flow

lps "capital commitment" --> fund
gp "manages" --> fund
gp "operates" --> mgmt
fund "deploys capital" --> deal
deal "invest" --> co_a
deal "invest" --> co_b
deal "invest" --> co_c
co_a --> exit
co_b --> exit
co_c --> exit
exit "proceeds" --> returns
returns "LP share (80%)" --> lps
returns --> carry
carry "GP carry" --> gp
mgmt "2% mgmt fee" --> deal

## Notes

- Check LP agreement terms before distribution {yellow}
- Due diligence typically takes 3-6 months {pink}
- Fund reporting: quarterly updates + annual meeting {blue}
