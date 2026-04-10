/// Bundled HRF template files embedded at compile time.
pub struct Template {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        name: "Architecture",
        category: "Engineering",
        description: "System architecture diagram",
        content: include_str!("engineering/architecture.spec"),
    },
    Template {
        name: "Data Pipeline",
        category: "Engineering",
        description: "ETL / data pipeline flow",
        content: include_str!("engineering/data-pipeline.spec"),
    },
    Template {
        name: "CI/CD Pipeline",
        category: "Engineering",
        description: "Build → test → deploy pipeline with quality gates",
        content: include_str!("engineering/ci-cd-pipeline.spec"),
    },
    Template {
        name: "State Machine",
        category: "Engineering",
        description: "Finite state machine with transitions (payment lifecycle example)",
        content: include_str!("engineering/state-machine.spec"),
    },
    Template {
        name: "Threat Model",
        category: "Engineering",
        description: "Security threat model",
        content: include_str!("engineering/threat-model.spec"),
    },
    Template {
        name: "Roadmap",
        category: "Strategy",
        description: "Product roadmap with phases",
        content: include_str!("strategy/roadmap.spec"),
    },
    Template {
        name: "GTM Strategy",
        category: "Strategy",
        description: "Go-to-market funnel diagram",
        content: include_str!("strategy/gtm-strategy.spec"),
    },
    Template {
        name: "User Journey",
        category: "Strategy",
        description: "User journey map with swimlanes",
        content: include_str!("strategy/user-journey.spec"),
    },
    Template {
        name: "OKRs",
        category: "Strategy",
        description: "Quarterly objectives + key results, rolled up from company mission",
        content: include_str!("strategy/okrs.spec"),
    },
    Template {
        name: "Org Chart",
        category: "Org",
        description: "Organizational chart",
        content: include_str!("org/org-chart.spec"),
    },
    Template {
        name: "Incident Map",
        category: "Ops",
        description: "Incident response flow",
        content: include_str!("ops/incident-map.spec"),
    },
    Template {
        name: "Team Topology",
        category: "Org",
        description: "Stream-aligned, platform, enabling, and subsystem teams",
        content: include_str!("org/team-topology.spec"),
    },
    Template {
        name: "RACI Matrix",
        category: "Org",
        description: "Responsibility assignment matrix",
        content: include_str!("org/raci-matrix.spec"),
    },
    Template {
        name: "Runbook",
        category: "Ops",
        description: "Operational incident runbook with severity triage",
        content: include_str!("ops/runbook.spec"),
    },
    Template {
        name: "On-Call Tree",
        category: "Ops",
        description: "On-call escalation tree",
        content: include_str!("ops/on-call-tree.spec"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specgraph::hrf::parse_hrf;

    #[test]
    fn test_all_templates_parse() {
        for template in TEMPLATES {
            let doc = parse_hrf(template.content).unwrap();
            assert!(
                !doc.nodes.is_empty(),
                "Template '{}' produced no nodes",
                template.name
            );
        }
    }

    #[test]
    fn test_templates_non_empty() {
        assert!(!TEMPLATES.is_empty());
    }
}
