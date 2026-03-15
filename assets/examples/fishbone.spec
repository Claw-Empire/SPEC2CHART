# Fishbone (Ishikawa) Diagram

Identify root causes of a problem by exploring 6 categories:
People, Process, Technology, Environment, Materials, Measurement.
The problem goes on the right; causes branch off the spine.

## Config
flow = LR
spacing = 80

## Problem
- [prob] High customer churn rate {risk} {critical} {icon:🎯} {glow}
  Effect / problem statement — place at the right end of the fishbone.

## People
- [p1] Inadequate support training {cause} {icon:👥}
  Support team lacks deep product knowledge.
- [p2] High staff turnover in CS {cause} {icon:👥}
- [p3] No clear escalation owner {cause} {icon:👥}

## Process
- [pr1] Onboarding takes 14 days — too slow {cause} {critical} {icon:⚙️}
  Industry benchmark is 3 days.
- [pr2] No proactive health-check process {cause} {icon:⚙️}
- [pr3] Renewal reminders sent too late {cause} {icon:⚙️}

## Technology
- [t1] Bug in export feature affecting 30% of users {cause} {critical} {icon:💻}
  P1 bug open for 3 sprints.
- [t2] Mobile app crashes on iOS 17 {cause} {icon:💻}
- [t3] Integration with Slack broken {cause} {icon:💻}

## Environment
- [e1] Increased competition — 3 new entrants {cause} {icon:🌍}
- [e2] Budget cuts at enterprise customers {cause} {icon:🌍}
- [e3] Economic downturn reduces SaaS spend {cause} {icon:🌍}

## Materials
- [m1] Outdated help documentation {cause} {icon:📄}
- [m2] No video tutorials for key workflows {cause} {icon:📄}
- [m3] In-app tooltips missing on advanced features {cause} {icon:📄}

## Measurement
- [ms1] No NPS tracking before renewal {cause} {icon:📊}
- [ms2] Churn detected only after the fact {cause} {icon:📊}
- [ms3] No leading indicators (usage drop alerts) {cause} {icon:📊}

## Flow
p1 --> prob: contributes
p2 --> prob: contributes
p3 --> prob: contributes
pr1 --> prob: contributes
pr2 --> prob: contributes
pr3 --> prob: contributes
t1 --> prob: contributes
t2 --> prob: contributes
t3 --> prob: contributes
e1 --> prob: contributes
e2 --> prob: contributes
e3 --> prob: contributes
m1 --> prob: contributes
m2 --> prob: contributes
m3 --> prob: contributes
ms1 --> prob: contributes
ms2 --> prob: contributes
ms3 --> prob: contributes
