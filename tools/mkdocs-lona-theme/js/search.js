document.addEventListener('DOMContentLoaded', function() {
  var trigger = document.getElementById('search-trigger');
  var modal = document.getElementById('search-modal');
  var container = document.getElementById('pagefind-container');
  var pagefindUI = null;

  if (!trigger || !modal || !container) {
    console.warn('Search elements not found');
    return;
  }

  function openSearch() {
    modal.hidden = false;
    document.body.style.overflow = 'hidden';

    // Initialize Pagefind on first open
    if (!pagefindUI && typeof PagefindUI !== 'undefined') {
      pagefindUI = new PagefindUI({
        element: container,
        showSubResults: true,
        showImages: false,
        excerptLength: 15,
      });
    }

    // Focus search input
    setTimeout(function() {
      var input = container.querySelector('.pagefind-ui__search-input');
      if (input) input.focus();
    }, 100);
  }

  function closeSearch() {
    modal.hidden = true;
    document.body.style.overflow = '';
  }

  // Click trigger
  trigger.addEventListener('click', openSearch);

  // Click backdrop to close
  modal.querySelector('.search-modal-backdrop').addEventListener('click', closeSearch);

  // Keyboard shortcuts
  document.addEventListener('keydown', function(e) {
    // Escape to close
    if (e.key === 'Escape' && !modal.hidden) {
      closeSearch();
      return;
    }

    // Cmd+K or Ctrl+K to open/close
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      if (modal.hidden) {
        openSearch();
      } else {
        closeSearch();
      }
    }
  });
});
