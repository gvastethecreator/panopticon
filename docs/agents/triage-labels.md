# Triage fields

GitHub Issues use one category label and one triage label. Local mirrors record the same values.

## Categories

| Canonical category | GitHub label  | Meaning                    |
| ------------------ | ------------- | -------------------------- |
| `bug`              | `bug`         | Existing behavior is wrong |
| `enhancement`      | `enhancement` | New behavior or improvement |

## Statuses

| Canonical status  | GitHub label      | Meaning                                   |
| ----------------- | ----------------- | ----------------------------------------- |
| `needs-triage`    | `needs-triage`    | Maintainer evaluation required            |
| `needs-info`      | `needs-info`      | Waiting for missing information           |
| `ready-for-agent` | `ready-for-agent` | Fully specified and ready for an AFK agent |
| `ready-for-human` | `ready-for-human` | Human implementation or judgment required |
| `wontfix`         | `wontfix`         | Deliberately not actioned                  |

## Project status

Project `9` uses the `Status` field with these exact values.

| Workflow state | Project value |
| -------------- | ------------- |
| Queued         | `Todo`        |
| Active         | `In Progress` |
| Finished       | `Done`        |

When triage changes, update the GitHub labels and the matching local fields. When execution changes, update the Project and local execution fields.

Local `Execution:` is separate from triage `Status:`. Use `queued`, `active`, `blocked`, or `finished`.

## Workflow labels

- `spec`: parent specification for implementation tickets.
- `wayfinder:map`: decision map for a large effort.

Create more workflow labels only when an enabled workflow requires them.
