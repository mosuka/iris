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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="getting_started.html"><strong aria-hidden="true">2.</strong> Getting Started</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="getting_started/installation.html"><strong aria-hidden="true">2.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="getting_started/quickstart.html"><strong aria-hidden="true">2.2.</strong> Quick Start</a></li></ol></li><li class="chapter-item expanded "><a href="concepts.html"><strong aria-hidden="true">3.</strong> Core Concepts</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="concepts/schema_and_fields.html"><strong aria-hidden="true">3.1.</strong> Schema &amp; Fields</a></li><li class="chapter-item expanded "><a href="concepts/analysis.html"><strong aria-hidden="true">3.2.</strong> Text Analysis</a></li><li class="chapter-item expanded "><a href="concepts/embedding.html"><strong aria-hidden="true">3.3.</strong> Embeddings</a></li><li class="chapter-item expanded "><a href="concepts/storage.html"><strong aria-hidden="true">3.4.</strong> Storage</a></li></ol></li><li class="chapter-item expanded "><a href="indexing.html"><strong aria-hidden="true">4.</strong> Indexing</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="indexing/lexical_indexing.html"><strong aria-hidden="true">4.1.</strong> Lexical Indexing</a></li><li class="chapter-item expanded "><a href="indexing/vector_indexing.html"><strong aria-hidden="true">4.2.</strong> Vector Indexing</a></li></ol></li><li class="chapter-item expanded "><a href="search.html"><strong aria-hidden="true">5.</strong> Search</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="search/lexical_search.html"><strong aria-hidden="true">5.1.</strong> Lexical Search</a></li><li class="chapter-item expanded "><a href="search/vector_search.html"><strong aria-hidden="true">5.2.</strong> Vector Search</a></li><li class="chapter-item expanded "><a href="search/hybrid_search.html"><strong aria-hidden="true">5.3.</strong> Hybrid Search</a></li><li class="chapter-item expanded "><a href="search/spelling_correction.html"><strong aria-hidden="true">5.4.</strong> Spelling Correction</a></li></ol></li><li class="chapter-item expanded "><a href="cli.html"><strong aria-hidden="true">6.</strong> CLI</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="cli/installation.html"><strong aria-hidden="true">6.1.</strong> Installation</a></li><li class="chapter-item expanded "><a href="cli/commands.html"><strong aria-hidden="true">6.2.</strong> Commands</a></li><li class="chapter-item expanded "><a href="cli/schema_format.html"><strong aria-hidden="true">6.3.</strong> Schema Format</a></li><li class="chapter-item expanded "><a href="cli/repl.html"><strong aria-hidden="true">6.4.</strong> REPL</a></li></ol></li><li class="chapter-item expanded "><a href="advanced.html"><strong aria-hidden="true">7.</strong> Advanced Features</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="advanced/query_dsl.html"><strong aria-hidden="true">7.1.</strong> Query DSL</a></li><li class="chapter-item expanded "><a href="advanced/id_management.html"><strong aria-hidden="true">7.2.</strong> ID Management</a></li><li class="chapter-item expanded "><a href="advanced/persistence.html"><strong aria-hidden="true">7.3.</strong> Persistence &amp; WAL</a></li><li class="chapter-item expanded "><a href="advanced/deletions.html"><strong aria-hidden="true">7.4.</strong> Deletions &amp; Compaction</a></li><li class="chapter-item expanded "><a href="advanced/error_handling.html"><strong aria-hidden="true">7.5.</strong> Error Handling</a></li><li class="chapter-item expanded "><a href="advanced/extensibility.html"><strong aria-hidden="true">7.6.</strong> Extensibility</a></li></ol></li><li class="chapter-item expanded "><a href="architecture.html"><strong aria-hidden="true">8.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="api_reference.html"><strong aria-hidden="true">9.</strong> API Reference</a></li></ol>';
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
