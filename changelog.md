# Changelog

How Prints work today and what's still missing: [prds/prints-status.md](prds/prints-status.md).

## [Unreleased]

### Added

- Only make Prints when I ask, in Settings > General > Prints. Prints stay on, but a thread asks before making one your message didn't ask for.
- Open on a reply's Print card opens it in the thread's side pane, which moves to each new version as it's saved.
- Print cards in replies on the website: published ones open their page, the rest say they're kept on the Mac.
- The guide covers keeping a Print fresh on a schedule.

### Changed

- In Ask before edits, a thread asks once per Print for updates instead of every time (Claude threads).

## 2026-11-11

Landed on main at `b221756c` and pushed to GitHub. Prints are still the `pages` experiment, off by default.

### Added

- Create diagram Prints from Mermaid source, edit their text, keep versions and download the source and SVG.
- Let outside-agent keys with read-all access list and read published Prints in bounded pages, without access to local unpublished Prints or any Print write operations.
- Behavior specs for diagram editing, invalid edits and project assignment in `features/prints.feature`, run by hand until the GPUI port has Prints.

### Changed

- Print cards use a solid native surface without a preview fade; diagram previews follow Threadbear's light and dark palettes.
- Prints were rebased onto Mike's 2026-11-11 release, keeping his new menu icons.

### Fixed

- Publishing, the Return key on the oldest waiting card and the Print wording in tool replies came back on main (`e8dd6683`) after they were lost when main was replaced with GitHub's merge of Mike's release.

## 2026-11-10

### Added

- Prints (called Pages until this day): stored, named, versioned pages a thread makes, in a project or none. Kinds: chart, dashboard, report, doc and bundle.
- A Print card in replies that follows the newest version.
- Prints on the project page, with Refresh, Edit with Agent, Edit Text for docs, Download, Versions, Projects and Delete with a 7-day restore.
- Download as PDF for the newest and each kept version.
- Publish a Print to your own Convex backend (API 73), opening it on the phone, the website's `/p/<slug>` page and for anyone with the link. Unpublish takes it down.
- The make-print and bundle-report skills.

### Changed

- Pages renamed to Prints everywhere people and agents see it. The old tool names and the page fence still work.
