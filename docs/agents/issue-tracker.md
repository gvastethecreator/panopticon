# Project tracker: GitHub and local mirrors

GitHub Issues and the linked GitHub Project hold live work state. Local Markdown files hold synchronized briefs, decisions, evidence, and handoffs.

## Identity

- Repository: `gvastethecreator/panopticon`
- Project owner: `gvastethecreator`
- Project number: `9`
- Project title: `Panopticon`
- Project URL: `https://github.com/users/gvastethecreator/projects/9`
- Local root: `.scratch/panopticon/`

## Authority

- GitHub owns Issue state, assignees, comments, dependencies, labels, and Project fields.
- Local files own expanded task context, decisions, verification evidence, and offline handoff notes.
- Shared fields must match across both surfaces.
- Add durable decisions and proof to `## Sync log`. Do not copy the full GitHub comment history.

## Local layout

- Specification: `.scratch/panopticon/spec.md` (`PRD.md` remains compatible).
- Ticket mirrors: `.scratch/panopticon/issues/<NN>-<slug>.md`.
- Rejected requests: `.scratch/panopticon/out-of-scope/<concept>.md`.
- Execution state: `.scratch/planning/`.
- Wayfinding mirrors: `.scratch/wayfinder/<effort-slug>/`.

Each ticket mirror starts with these fields:

```markdown
# <NN>: <title>

GitHub issue: <url-or-pending>
GitHub project: https://github.com/users/gvastethecreator/projects/9
Sync: pending | synced | conflict
Last synced: <ISO-8601-or-never>
Remote updated: <ISO-8601-or-unknown>
Category: bug | enhancement
Status: needs-triage | needs-info | ready-for-agent | ready-for-human | wontfix
Project status: Todo | In Progress | Done
Execution: queued | active | blocked | finished
Type: AFK | HITL
Source: <spec path, issue URL, or conversation>
Blocked by: <GitHub issue numbers or None>
```

## Sync protocol

1. Read the Issue, Project item, and local mirror before a change.
2. If both surfaces changed after `Last synced`, set `Sync: conflict` and stop.
3. Write the local draft with `Sync: pending` before remote creation.
4. Create or update the GitHub Issue. Add native parent and blocking relationships when they apply.
5. Add the Issue to Project `9` under `gvastethecreator`.
6. Set the Project `Status` field to the configured value.
7. Update identifiers, shared fields, timestamps, and `Sync: synced` in the local mirror.
8. If a step fails, record it under `## Sync log`. Retry from the stored Issue URL.

Never create a second Issue because a later sync step failed.

## GitHub commands

Use the exact identities from this document.

```powershell
gh issue view <number> -R gvastethecreator/panopticon --json number,title,state,body,labels,assignees,comments,updatedAt,url
gh project view 9 --owner gvastethecreator --format json
gh project field-list 9 --owner gvastethecreator --format json
gh project item-list 9 --owner gvastethecreator --limit 200 --format json --field Status
gh project item-add 9 --owner gvastethecreator --url <issue-url>
gh project item-edit 9 --owner gvastethecreator --url <issue-url> --field Status --value <configured-value>
gh issue create -R gvastethecreator/panopticon --title <title> --body-file <path> --parent <parent-number> --blocked-by <number,number>
gh issue edit <issue-number> -R gvastethecreator/panopticon --parent <parent-number> --add-blocked-by <number>
```

Omit `--parent` or `--blocked-by` when the relationship does not apply.

## Triage and implementation

- A triage change updates one category label, one triage label, and the matching local fields.
- Implementation start assigns the Issue and sets both execution surfaces to active.
- Verified completion records proof, closes the Issue, and sets both execution surfaces to finished.
- A blocker keeps the Issue open. Record the blocker on GitHub and set local `Execution: blocked`.

## Wayfinding operations

- Create a map Issue with `wayfinder:map`. Mirror it at `.scratch/wayfinder/<effort-slug>/map.md`.
- Create decision tickets as native sub-issues. Mirror them under `.scratch/wayfinder/<effort-slug>/tickets/`.
- Use native blocking relationships. Mirror the same Issue numbers in `Blocked by:`.
- Claim a ticket with an assignee, active Project status, and local `Execution: active`.
- Resolve a ticket with a GitHub comment, a closed Issue, and local proof.
