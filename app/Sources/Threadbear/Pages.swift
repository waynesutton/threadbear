import AppKit
import ThreadbearCore

/// Pages (docs/protocol.md, "Pages"): the pages agents make, kept by the
/// runner, listed on each project's page with a menu for each: Open,
/// Refresh, Edit with Agent, Edit Text (docs), Download, Versions, Move to
/// Project and Delete. The pages experiment.
extension AppModel {
    var pagesOn: Bool { experiments.contains(Page.experiment) }

    /// The list after hello; `pages.updated` keeps it current from then on.
    func loadPages() {
        guard let client, pagesOn else {
            pages = []
            return
        }
        Task {
            do {
                pages = try await client.listPages()
            } catch {
                // An older runner, or the experiment just turned off.
                pages = []
                AppLog.write("rpc", "pages.list failed: \(error.localizedDescription)")
            }
        }
    }

    /// Deleted ones aren't in `pages.updated`: asked for when a project's
    /// Recently Deleted is shown, and after a delete or restore.
    func loadDeletedPages() {
        guard let client, pagesOn else { return }
        Task { deletedPages = (try? await client.listPages(deleted: true)) ?? [] }
    }

    func pages(in projectId: String) -> [Page] {
        pages.filter { $0.isIn(project: projectId) }.sorted { $0.updatedAt > $1.updatedAt }
    }

    func deletedPages(in projectId: String) -> [Page] {
        deletedPages.filter { $0.isIn(project: projectId) }
    }

    /// Opens a version (the newest by default) in a window of its own, the
    /// way a reply's page opens: sandboxed, scripts off until Run them.
    func openPage(_ page: Page, version: Int? = nil) {
        guard let client else { return }
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                let got = try await client.getPage(page.id, version: version)
                let v = version ?? got.page.version
                RichBlockWindowController.show(URL(fileURLWithPath: got.file), id: "page:\(page.id):\(v)", title: got.page.title, subtitle: v == got.page.version ? "Version \(v)" : "Version \(v) of \(got.page.version)")
            } catch {
                show(error)
            }
        }
    }

    /// Sends the page to the thread that made it to be made again from its
    /// recipe, and opens that thread so its work can be watched.
    func refreshPage(_ page: Page) {
        guard let client else { return }
        whileBusy(BusyKey.page(page.id), note: "Asking a thread to refresh \(page.title)") { [self] in
            do {
                let sent = try await client.refreshPage(page.id)
                openThread(sent.threadId)
            } catch {
                show(error)
            }
        }
    }

    func editPageWithAgent(_ page: Page, text: String) {
        guard let client, !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return }
        whileBusy(BusyKey.page(page.id), note: "Asking a thread to change \(page.title)") { [self] in
            do {
                let sent = try await client.editPageWithAgent(page.id, text: text)
                openThread(sent.threadId)
            } catch {
                show(error)
            }
        }
    }

    /// A doc's markdown for the editor sheet, or nil (shown as an error).
    func pageMarkdown(_ page: Page) async -> String? {
        guard let client else { return nil }
        do {
            return try await client.getPage(page.id).markdown ?? ""
        } catch {
            show(error)
            return nil
        }
    }

    /// Saves the user's edit as a new version; `done` runs once it's in.
    func savePageDoc(_ page: Page, markdown: String, done: @escaping @MainActor (Bool) -> Void) {
        guard let client else { return done(false) }
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                replace(page: try await client.savePageDoc(page.id, markdown: markdown))
                done(true)
            } catch {
                show(error)
                done(false)
            }
        }
    }

    func restorePageVersion(_ page: Page, version: Int) {
        guard let client else { return }
        whileBusy(BusyKey.page(page.id), note: "Restoring version \(version) of \(page.title)") { [self] in
            do {
                replace(page: try await client.restorePageVersion(page.id, version: version))
            } catch {
                show(error)
            }
        }
    }

    /// Takes the row away at once; the runner keeps it 7 days.
    func deletePage(_ page: Page) {
        guard let client else { return }
        let before = pages
        pages.removeAll { $0.id == page.id }
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                let gone = try await client.deletePage(page.id)
                deletedPages.removeAll { $0.id == gone.id }
                deletedPages.insert(gone, at: 0)
            } catch {
                pages = before
                show(error)
            }
        }
    }

    func restorePage(_ page: Page) {
        guard let client else { return }
        deletedPages.removeAll { $0.id == page.id }
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                replace(page: try await client.restorePage(page.id))
            } catch {
                loadDeletedPages()
                show(error)
            }
        }
    }

    /// Puts the page in or out of a project, showing the change at once.
    func togglePage(_ page: Page, inProject projectId: String) {
        guard let client else { return }
        let adding = !page.isIn(project: projectId)
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                replace(page: try await client.setPageProjects(page.id, add: adding ? [projectId] : [], remove: adding ? [] : [projectId]))
            } catch {
                show(error)
            }
        }
    }

    /// Asks where, copies the page's files into a new folder named for it
    /// there, and shows it in the Finder.
    func downloadPage(_ page: Page, version: Int? = nil) {
        guard let client else { return }
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.prompt = "Save Here"
        panel.message = "Choose where to save \"\(page.title)\". It goes in a new folder with its index.html, which opens in any browser."
        guard panel.runModal() == .OK, let folder = panel.url else { return }
        let name = page.title.replacingOccurrences(of: "/", with: "-").replacingOccurrences(of: ":", with: "-")
        var dest = folder.appendingPathComponent(name)
        var n = 2
        while FileManager.default.fileExists(atPath: dest.path) {
            dest = folder.appendingPathComponent("\(name) \(n)")
            n += 1
        }
        let target = dest
        whileBusy(BusyKey.page(page.id), note: "Saving \(page.title)") { [self] in
            do {
                let file = try await client.exportPage(page.id, to: target.path, version: version)
                NSWorkspace.shared.activateFileViewerSelecting([URL(fileURLWithPath: file)])
            } catch {
                show(error)
            }
        }
    }

    private func replace(page: Page) {
        if let i = pages.firstIndex(where: { $0.id == page.id }) { pages[i] = page } else if page.deletedAt == nil { pages.insert(page, at: 0) }
    }
}
