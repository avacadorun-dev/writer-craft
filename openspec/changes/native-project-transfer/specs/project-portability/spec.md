# Native project portability interaction delta

## ADDED Requirements

### Requirement: Guarded whole-project transfer

The desktop SHALL offer complete-project native archive and readable-folder export with a fixed
visible project identity, and independent native restore using one checked private input snapshot.
Native File/settings actions SHALL distinguish project transfers from whole-workspace backups.

#### Scenario: Scoped pending note review

- WHEN current-project notes require explicit review before transfer
- THEN only actual project-owned pending documents are offered, unrelated parked drafts remain
  untouched, and Review/Cancel can return to the captured manuscript or board context.

#### Scenario: Exact return context

- WHEN the review returns to unchanged acknowledged source
- THEN a shared bounded presentation bookmark restores typed selection, actual measured manuscript
  scroll or board camera/selection after matching scope, saved revision and content identity;
  changed source opens its latest data with localized disclosure rather than quote matching.

#### Scenario: Cancel or acknowledgement

- WHEN cancellation races with an export/restore acknowledgement
- THEN confirmed publication remains completed, unacknowledged outcomes remain explicitly
  unconfirmed, the actual target remains visible and retries retain their input without replacing
  unrelated files or automatically reopening the restore source path.

#### Scenario: Complete native controls

- WHEN a transfer is configured or completed in any supported locale
- THEN the dialog has bounded content scrolling, shared40px fields/actions, shared24px insets and a
  stable reachable footer; invalid names preserve input, unavailable scope is not offered as an
  enabled selector, and the new project/output path remains available after the modal is closed.
