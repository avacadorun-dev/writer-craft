# Owner findings and GitHub issue workflow

Effective 2026-11-08. Repository: [avacadorun-dev/writer-craft](https://github.com/avacadorun-dev/writer-craft). The owner wants every future actionable finding recorded in English from his own account, with a useful investigation history. This standing authorization covers routine issue creation and updates; do not ask again simply because an issue is public. Preserve the distinction between recording a finding, selecting release scope and publishing a release.

## Identity and duplicate check

1. Read the owner's report, affected version/build, platform and available evidence. Separate a broken existing behavior from deferred legacy-parity functionality.
2. Search open and closed repository issues using the symptom, affected component and distinctive reproduction terms. Read plausible matches before creating anything. Reuse an existing issue for the same finding; link a previously closed issue when reporting a verified regression. Avoid one duplicate per chat message or per failed retest.
3. Confirm the active GitHub identity is the owner's authenticated `avacadorun-dev` account. Prefer its existing authenticated `gh` session or authorized connector/API. Browser interaction is a last resort, not the normal route for repeated issue work. Do not configure a bot identity, substitute a different logged-in account, invent an author signature or describe an issue as owner-authored if another identity actually posted it. An unavailable account is an authentication blocker; retain the ready draft and explain the blocker.
4. Write a concrete English issue and apply `bug` for failed existing promised behavior or `enhancement` for deferred parity/capability work. Record why a classification matters if the same request could be mistaken for a current-release bug.

The automatic recording instruction applies to owner findings. Agent-only hypotheses need evidence and must not be presented as an owner observation. Source author identity and the person performing an investigation are separate facts; preserve both accurately.

## Tool routing

Use `gh` or a connected GitHub API when available. Query the current viewer and repository issues directly, use structured responses for duplicate checks, and submit multiline text through a structured body argument or an exact UTF-8 temporary file with `gh issue create --body-file`. Keep tokens and authentication material out of commands, issue bodies and logs. Git SSH access alone does not provide GitHub API authentication.

When API authentication is unavailable, prepare the complete sanitized issue and report the one-time authentication requirement. Do not spend repeated browser interactions recreating routine API operations. An existing browser session is a last-resort fallback when authorized and practical; it does not change the required owner identity.

## Issue body

Use this structure for a reproducible bug, retaining the same issue as the investigation develops:

```markdown
## Summary
<One concrete symptom and its effect on the author.>

## Report and environment
- Source: owner report, <date>.
- App version / build: <verified value, or unknown>.
- Platform: <OS and architecture actually observed>.
- Source / CI / artifact: <permalinks if available>.

## Steps to reproduce
1. <Start from a named state with synthetic data.>
2. <Perform the triggering action.>
3. <Observe the result.>

## Expected behavior
<The applicable product contract and visible result.>

## Actual behavior
<What was observed, including relevant error and scope.>

## Evidence and investigation history
- <Date>: <observation/retest>, <sanitized screenshot or verification link>.

## Acceptance checks
- [ ] <Reproduce on the affected build, or state why it is unconfirmed.>
- [ ] <Verify the corrected behavior and the relevant regression.>
```

For an enhancement, retain Expected/Actual and history, replace reproduction with the author's workflow, link the existing source-parity contract, and state current-release exclusion and proposed acceptance. Do not promise a version or delivery date before scope is chosen. Current transient text selection and future persistent note annotations must be described separately.

## Evidence, updates and closure

Use synthetic excerpts. Sanitize unpublished manuscripts, note contents, provider keys, session tokens, account details and machine-specific paths before posting logs or screenshots. A public reproduction must remain useful after sanitization; link internal evidence only where the owner has authorized that destination and access.

Append meaningful dated findings, root cause, linked fix/PR/commit and actual verification. Preserve the original expected/actual report rather than rewriting it into a different problem. Do not close a bug because a compiler passed or a plausible fix exists. Record the build and concrete retest that resolved it; otherwise leave verification pending. An enhancement's specification and issue do not imply implementation. Link each issue back into the relevant change or release planning record once its URL is known.

Closing a verified finding follows the owner's authorized workflow. Tags, draft releases and publication continue to follow [RELEASING.md](RELEASING.md); issue authorization is not blanket publication authority.

## Required completion audit

Run an issue audit after each implementation task and before each release candidate. Read linked issues plus the current open list, for example with `gh issue list --state open --limit 100 --json number,title,url`; paginate when needed. Check the actual issue body and acceptance criteria rather than relying on its title or an old local status.

For fully verified work, append a concise ready-for-owner-review comment with the fix commit, build/version, relevant native/package evidence and any remaining scope limitation. Mark only verified acceptance checkboxes complete. The owner's latest alpha.2 instruction requires product issues to stay open until the final candidate has been checked and accepted by the owner. After that explicit acceptance, close only the accepted, fully verified issues with reason `completed` under the verified owner account, without a redundant confirmation. Do not add automatic closing keywords while acceptance is pending.

For partial or unverified work, leave the issue open and state the exact outstanding check. A planned feature, generated specification, passing compilation, available archive or published release is not by itself completion evidence. Reuse/reopen the existing issue when a confirmed regression invalidates its resolution. Do not use automatic closing keywords for a change whose issue acceptance is still pending.

Record the audit outcome in the task/release verification or handoff: issues closed, issues still open and the concrete reason. An audit does not select the next release scope or authorize implementing deferred features.

## Initial tracked findings

- [Issue #1: macOS application/Dock icon behavior](https://github.com/avacadorun-dev/writer-craft/issues/1) — owner-authored account `avacadorun-dev`, labeled `bug`. Link actual fix and packaged-app retest evidence before considering it resolved.
- [Persistent selected-passage marks and attached notes — issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2) was created through authenticated GitHub CLI as `avacadorun-dev`. It is included in the owner-approved alpha.2 editor scope; core work is partly qualified and native marks/notes acceptance remains pending.

Tracking: [persistent text marks and author notes — issue #2](https://github.com/avacadorun-dev/writer-craft/issues/2).

On 2026-11-08, official GitHub CLI 2.103.3 was installed for the development environment and authenticated through its device flow. The actor was verified through `gh api user`; no API credential is stored in this repository. This is a contributor tool, not an application runtime dependency.

## Administrative edits

Routine changes to the project license, repository About/topics, release housekeeping or contributor tools are direct repository work. Do not create an issue for them unless the owner explicitly asks. The owner specifically requested changing the project to MIT without opening a license issue. This exception does not remove issue history for reported product bugs or enhancements.

## Approved alpha.2 work breakdown

The owner explicitly included all open product issues on 2026-11-08, superseding prior deferrals. The editor epic #4 remains complete, with #7 authoring, #8 syntax, #9 controls, #6 layout and #2 notes. Alpha.2 now also includes #3 installers, #10 complete source-project/Canvas/media compatibility and #11 transparent data/portable Markdown plus artifacts. Issue #1 was already closed after direct owner confirmation of the Dock icon. See [NEXT_RELEASE.md](NEXT_RELEASE.md).

Keep every remaining product issue open until owner acceptance of the complete candidate, even after independent verification makes it ready to close. Continue updating exact source/build evidence and acceptance checklists. Routine administrative work still does not require a separate issue.


## Latest scheduling decision — checkpoint alpha.2, remaining work alpha.3

The owner subsequently authorized publishing the completed coherent checkpoint as alpha.2 and moving all unfinished large blocks to alpha.3. This supersedes the earlier scheduling statements above. Preserve original issue requirements and progress; use milestones and a current status note rather than claiming missing work is complete. In particular, keep the full-editor epic and incomplete project/media/Canvas/restore work open for alpha.3. Publishing alpha.2 alone never satisfies an issue or grants final owner acceptance.
