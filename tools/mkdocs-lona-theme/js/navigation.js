// Scroll spy for TOC - highlights current section as user scrolls
document.addEventListener('DOMContentLoaded', function() {
  var content = document.querySelector('.content');
  var tocLinks = document.querySelectorAll('#page-toc .sidebar-link');

  if (!content || tocLinks.length === 0) return;

  // Get all h2 headings
  var headings = content.querySelectorAll('h2[id]');
  if (headings.length === 0) return;

  function updateActiveSection() {
    var current = '';

    headings.forEach(function(heading) {
      var rect = heading.getBoundingClientRect();
      var contentRect = content.getBoundingClientRect();
      var relativeTop = rect.top - contentRect.top;

      // Consider a heading "current" if it's within 120px of the top of the content area
      if (relativeTop <= 120) {
        current = heading.getAttribute('id');
      }
    });

    tocLinks.forEach(function(link) {
      link.classList.remove('active');
      var href = link.getAttribute('href');
      if (href === '#' + current) {
        link.classList.add('active');
      }
    });
  }

  content.addEventListener('scroll', updateActiveSection);
  updateActiveSection(); // Initial call
});
