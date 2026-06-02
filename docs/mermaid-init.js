// Ferro mdBook Mermaid initialization
// Replaces mdbook-mermaid preprocessor with client-side rendering
(function() {
    document.addEventListener('DOMContentLoaded', function() {
        // Find all code blocks with language-mermaid and convert them
        document.querySelectorAll('pre code.language-mermaid').forEach(function(block) {
            var pre = block.parentElement;
            var container = document.createElement('div');
            container.classList.add('mermaid');
            container.textContent = block.textContent;
            pre.replaceWith(container);
        });

        // Detect theme
        var darkThemes = ['ayu', 'navy', 'coal'];
        var isLight = true;
        for (var i = 0; i < document.documentElement.classList.length; i++) {
            if (darkThemes.indexOf(document.documentElement.classList[i]) !== -1) {
                isLight = false;
                break;
            }
        }

        mermaid.initialize({
            startOnLoad: false,
            theme: isLight ? 'default' : 'dark'
        });
        mermaid.run();
    });
})();
