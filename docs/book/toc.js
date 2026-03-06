// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded affix "><li class="part-title">Getting Started</li><li class="chapter-item expanded "><a href="getting_started.html"><strong aria-hidden="true">2.</strong> Getting Started</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/installation.html"><strong aria-hidden="true">2.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting_started/quickstart.html"><strong aria-hidden="true">2.2.</strong> Quick Start</a></li><li class="chapter-item expanded "><a href="getting_started/examples.html"><strong aria-hidden="true">2.3.</strong> Examples</a></li></ol></li><li class="chapter-item expanded "><a href="architecture.html"><strong aria-hidden="true">3.</strong> Architecture Overview</a></li><li class="chapter-item expanded affix "><li class="part-title">Core Concepts</li><li class="chapter-item expanded "><a href="concepts/schema_and_fields.html"><strong aria-hidden="true">4.</strong> Schema &amp; Fields</a></li><li class="chapter-item expanded "><a href="concepts/analysis.html"><strong aria-hidden="true">5.</strong> Text Analysis</a></li><li class="chapter-item expanded "><a href="concepts/embedding.html"><strong aria-hidden="true">6.</strong> Embeddings</a></li><li class="chapter-item expanded "><a href="concepts/storage.html"><strong aria-hidden="true">7.</strong> Storage</a></li><li class="chapter-item expanded "><a href="concepts/indexing.html"><strong aria-hidden="true">8.</strong> Indexing</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="concepts/indexing/lexical_indexing.html"><strong aria-hidden="true">8.1.</strong> Lexical Indexing</a></li><li class="chapter-item expanded "><a href="concepts/indexing/vector_indexing.html"><strong aria-hidden="true">8.2.</strong> Vector Indexing</a></li></ol></li><li class="chapter-item expanded "><a href="concepts/search.html"><strong aria-hidden="true">9.</strong> Search</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="concepts/search/lexical_search.html"><strong aria-hidden="true">9.1.</strong> Lexical Search</a></li><li class="chapter-item expanded "><a href="concepts/search/vector_search.html"><strong aria-hidden="true">9.2.</strong> Vector Search</a></li><li class="chapter-item expanded "><a href="concepts/search/hybrid_search.html"><strong aria-hidden="true">9.3.</strong> Hybrid Search</a></li></ol></li><li class="chapter-item expanded "><a href="concepts/query_dsl.html"><strong aria-hidden="true">10.</strong> Query DSL</a></li><li class="chapter-item expanded affix "><li class="part-title">laurus (Library)</li><li class="chapter-item expanded "><a href="laurus/overview.html"><strong aria-hidden="true">11.</strong> Library Overview</a></li><li class="chapter-item expanded "><a href="laurus/engine.html"><strong aria-hidden="true">12.</strong> Engine</a></li><li class="chapter-item expanded "><a href="laurus/scoring.html"><strong aria-hidden="true">13.</strong> Scoring &amp; Ranking</a></li><li class="chapter-item expanded "><a href="laurus/faceting.html"><strong aria-hidden="true">14.</strong> Faceting</a></li><li class="chapter-item expanded "><a href="laurus/highlighting.html"><strong aria-hidden="true">15.</strong> Highlighting</a></li><li class="chapter-item expanded "><a href="laurus/spelling_correction.html"><strong aria-hidden="true">16.</strong> Spelling Correction</a></li><li class="chapter-item expanded "><a href="laurus/id_management.html"><strong aria-hidden="true">17.</strong> ID Management</a></li><li class="chapter-item expanded "><a href="laurus/persistence.html"><strong aria-hidden="true">18.</strong> Persistence &amp; WAL</a></li><li class="chapter-item expanded "><a href="laurus/deletions.html"><strong aria-hidden="true">19.</strong> Deletions &amp; Compaction</a></li><li class="chapter-item expanded "><a href="laurus/error_handling.html"><strong aria-hidden="true">20.</strong> Error Handling</a></li><li class="chapter-item expanded "><a href="laurus/extensibility.html"><strong aria-hidden="true">21.</strong> Extensibility</a></li><li class="chapter-item expanded "><a href="laurus/api_reference.html"><strong aria-hidden="true">22.</strong> API Reference</a></li><li class="chapter-item expanded affix "><li class="part-title">laurus-cli (CLI)</li><li class="chapter-item expanded "><a href="cli/overview.html"><strong aria-hidden="true">23.</strong> CLI Overview</a></li><li class="chapter-item expanded "><a href="cli/installation.html"><strong aria-hidden="true">24.</strong> Installation</a></li><li class="chapter-item expanded "><a href="cli/commands.html"><strong aria-hidden="true">25.</strong> Commands</a></li><li class="chapter-item expanded "><a href="cli/schema_format.html"><strong aria-hidden="true">26.</strong> Schema Format</a></li><li class="chapter-item expanded "><a href="cli/repl.html"><strong aria-hidden="true">27.</strong> REPL</a></li><li class="chapter-item expanded affix "><li class="part-title">laurus-server (Server)</li><li class="chapter-item expanded "><a href="server/overview.html"><strong aria-hidden="true">28.</strong> Server Overview</a></li><li class="chapter-item expanded "><a href="server/getting_started.html"><strong aria-hidden="true">29.</strong> Getting Started</a></li><li class="chapter-item expanded "><a href="server/configuration.html"><strong aria-hidden="true">30.</strong> Configuration</a></li><li class="chapter-item expanded "><a href="server/grpc_api.html"><strong aria-hidden="true">31.</strong> gRPC API Reference</a></li><li class="chapter-item expanded "><a href="server/http_gateway.html"><strong aria-hidden="true">32.</strong> HTTP Gateway</a></li><li class="chapter-item expanded affix "><li class="part-title">Development Guide</li><li class="chapter-item expanded "><a href="development/build_and_test.html"><strong aria-hidden="true">33.</strong> Build &amp; Test</a></li><li class="chapter-item expanded "><a href="development/feature_flags.html"><strong aria-hidden="true">34.</strong> Feature Flags</a></li><li class="chapter-item expanded "><a href="development/project_structure.html"><strong aria-hidden="true">35.</strong> Project Structure</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
