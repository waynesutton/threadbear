import AppKit
import ThreadbearCore

/// Pages (docs/protocol.md, "Pages"): the pages agents make, kept by the
/// runner, listed on each project's page. The pages experiment.
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

    func pages(in projectId: String) -> [Page] {
        pages.filter { $0.isIn(project: projectId) }.sorted { $0.updatedAt > $1.updatedAt }
    }

    /// Opens a page's newest version in a window of its own, the way a
    /// reply's page opens: sandboxed, scripts off until Run them.
    func openPage(_ page: Page) {
        guard let client else { return }
        whileBusy(BusyKey.page(page.id)) { [self] in
            do {
                let got = try await client.getPage(page.id)
                RichBlockWindowController.show(URL(fileURLWithPath: got.file), id: "page:\(page.id):\(got.page.version)", title: got.page.title, subtitle: "Version \(got.page.version)")
            } catch {
                show(error)
            }
        }
    }
}
