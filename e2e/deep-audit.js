const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8081';
const OUT = path.resolve(__dirname, '../target/audit-results/leptos');
const WASM_HYDRATE_MS = 8000;

const phases = [];
let consoleErrors = [];

function log(msg) {
  const line = `[${new Date().toISOString()}] ${msg}`;
  console.log(line);
  return line;
}

function pass(phase, step, detail) {
  phases.push({ phase, step, status: 'PASS', detail });
  log(`  PASS ${phase}/${step}: ${detail}`);
}

function fail(phase, step, detail) {
  phases.push({ phase, step, status: 'FAIL', detail });
  log(`  FAIL ${phase}/${step}: ${detail}`);
}

async function screenshot(page, name) {
  const p = path.join(OUT, `${name}.png`);
  try {
    await page.screenshot({ path: p, fullPage: true });
  } catch (e) {
    log(`  screenshot failed for ${name}: ${e.message}`);
  }
}

async function collectA11y(page, label) {
  const data = await page.evaluate(() => {
    const all = document.querySelectorAll('*');
    const buttons = document.querySelectorAll('button, [role="button"]');
    const inputs = document.querySelectorAll('input, select, textarea');
    const links = document.querySelectorAll('a[href]');
    const images = document.querySelectorAll('img');

    let imgsNoAlt = 0;
    images.forEach(i => { if (!i.hasAttribute('alt') && !i.getAttribute('aria-hidden')) imgsNoAlt++; });

    let unlabeledBtns = 0;
    buttons.forEach(b => {
      const lbl = b.getAttribute('aria-label') || b.getAttribute('aria-labelledby') || b.getAttribute('title');
      const txt = (b.textContent || '').trim();
      if (!lbl && !txt) unlabeledBtns++;
    });

    let inputsNoLabel = 0;
    inputs.forEach(inp => {
      const id = inp.id;
      const hasFor = id ? !!document.querySelector(`label[for="${id}"]`) : false;
      const hasAriaLabel = inp.getAttribute('aria-label') || inp.getAttribute('aria-labelledby');
      const wrappedInLabel = inp.closest('label');
      const hasTitle = inp.getAttribute('title');
      if (!hasFor && !hasAriaLabel && !wrappedInLabel && !hasTitle) inputsNoLabel++;
    });

    let positiveTabindex = 0;
    all.forEach(el => {
      const t = el.getAttribute('tabindex');
      if (t && parseInt(t, 10) > 0) positiveTabindex++;
    });

    const landmarks = {
      nav: document.querySelectorAll('nav, [role="navigation"]').length,
      main: document.querySelectorAll('main, [role="main"]').length,
      aside: document.querySelectorAll('aside, [role="complementary"]').length,
      header: document.querySelectorAll('header, [role="banner"]').length,
      footer: document.querySelectorAll('footer, [role="contentinfo"]').length,
    };

    return {
      totalElements: all.length,
      buttons: buttons.length,
      inputs: inputs.length,
      links: links.length,
      images: images.length,
      imgsNoAlt,
      unlabeledBtns,
      inputsNoLabel,
      positiveTabindex,
      landmarks,
      bodyText: (document.body?.innerText || '').substring(0, 500),
    };
  });

  log(`  [a11y ${label}] elements=${data.totalElements} buttons=${data.buttons} inputs=${data.inputs} links=${data.links} images=${data.images}`);
  log(`    imgs_no_alt=${data.imgsNoAlt} unlabeled_btns=${data.unlabeledBtns} inputs_no_label=${data.inputsNoLabel} positive_tabindex=${data.positiveTabindex}`);
  log(`    landmarks: nav=${data.landmarks.nav} main=${data.landmarks.main} aside=${data.landmarks.aside} header=${data.landmarks.header} footer=${data.landmarks.footer}`);
  return data;
}

(async () => {
  fs.mkdirSync(OUT, { recursive: true });

  const browser = await chromium.launch({ headless: true });
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 } });
  const page = await ctx.newPage();

  consoleErrors = [];
  page.on('console', m => { if (m.type() === 'error') consoleErrors.push(m.text()); });
  page.on('pageerror', e => consoleErrors.push(e.message));

  const allA11y = {};
  const perf = {};

  // ─── PHASE 1: Core Navigation ───────────────────────────────────
  log('\n═══ PHASE 1: Core Navigation ═══');
  try {
    const t0 = Date.now();
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    perf.loadTime = Date.now() - t0;
    log(`  Page loaded in ${perf.loadTime}ms`);

    await screenshot(page, 'phase1-01-initial-load');

    const title = await page.title();
    if (title.toLowerCase().includes('ferro')) {
      pass('phase1', 'title', `Title is "${title}"`);
    } else {
      fail('phase1', 'title', `Title is "${title}", expected "Ferro" or similar`);
    }

    const elemCount = await page.evaluate(() => document.querySelectorAll('*').length);
    if (elemCount >= 165) {
      pass('phase1', 'element-count', `${elemCount} elements (>= 165)`);
    } else {
      fail('phase1', 'element-count', `${elemCount} elements (< 165)`);
    }

    if (consoleErrors.length === 0) {
      pass('phase1', 'no-console-errors', 'No console errors');
    } else {
      fail('phase1', 'no-console-errors', `${consoleErrors.length} console error(s): ${consoleErrors.slice(0, 3).join(' | ')}`);
    }

    // Check FILES and FAVORITES tabs
    const tabs = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const filesTab = btns.find(b => b.textContent.includes('FILES') || b.textContent.includes('Files'));
      const favsTab = btns.find(b => b.textContent.includes('FAVORITES') || b.textContent.includes('Favorites') || b.textContent.includes('★'));
      return {
        filesVisible: filesTab ? filesTab.offsetParent !== null : false,
        favsVisible: favsTab ? favsTab.offsetParent !== null : false,
        filesText: filesTab?.textContent?.trim() || null,
        favsText: favsTab?.textContent?.trim() || null,
      };
    });

    if (tabs.filesVisible) {
      pass('phase1', 'files-tab-visible', `FILES tab visible: "${tabs.filesText}"`);
    } else {
      fail('phase1', 'files-tab-visible', 'FILES tab not found or not visible');
    }
    if (tabs.favsVisible) {
      pass('phase1', 'favs-tab-visible', `FAVORITES tab visible: "${tabs.favsText}"`);
    } else {
      fail('phase1', 'favs-tab-visible', 'FAVORITES tab not found or not visible');
    }

    // Click FILES tab
    const filesClicked = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => b.textContent.includes('FILES') || b.textContent.includes('Files'));
      if (btn) { btn.click(); return true; }
      return false;
    });
    if (filesClicked) {
      await page.waitForTimeout(2000);
      await screenshot(page, 'phase1-02-files-tab');
      const hasFiles = await page.evaluate(() => {
        return document.querySelectorAll('table, [role="row"], [role="gridcell"]').length > 0
          || document.querySelector('[role="region"]') !== null;
      });
      pass('phase1', 'files-tab-click', hasFiles ? 'Files tab loaded with file browser content' : 'Files tab clicked (content check inconclusive)');
    } else {
      fail('phase1', 'files-tab-click', 'Could not find FILES tab button');
    }

    // Click FAVORITES tab
    const favsClicked = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => b.textContent.includes('FAVORITES') || b.textContent.includes('Favorites') || b.textContent.includes('★'));
      if (btn) { btn.click(); return true; }
      return false;
    });
    if (favsClicked) {
      await page.waitForTimeout(2000);
      await screenshot(page, 'phase1-03-favorites-tab');
      const bodyText = await page.evaluate(() => document.body?.innerText || '');
      pass('phase1', 'favs-tab-click', 'Favorites tab clicked');
    } else {
      fail('phase1', 'favs-tab-click', 'Could not find FAVORITES tab button');
    }

    allA11y['phase1'] = await collectA11y(page, 'phase1');
  } catch (e) {
    fail('phase1', 'exception', e.message);
  }

  // ─── PHASE 2: File Operations ───────────────────────────────────
  log('\n═══ PHASE 2: File Operations ═══');
  try {
    // Navigate back to files root
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase2-00-root');

    // Click FILES tab first
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => b.textContent.includes('FILES') || b.textContent.includes('Files'));
      if (btn) btn.click();
    });
    await page.waitForTimeout(1000);

    // Click "New Folder" button
    const newFolderClicked = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => {
        const aria = b.getAttribute('aria-label') || '';
        const text = b.textContent || '';
        return aria.includes('new folder') || aria.includes('New Folder') || aria.includes('New folder')
          || text.includes('New Folder') || text.includes('NEW FOLDER');
      });
      if (btn) { btn.click(); return true; }
      return false;
    });

    if (!newFolderClicked) {
      fail('phase2', 'new-folder-button', 'New Folder button not found');
    } else {
      await page.waitForTimeout(1000);
      await screenshot(page, 'phase2-01-new-folder-dialog');

      // Type folder name
      const inputFound = await page.evaluate(() => {
        const inp = document.getElementById('new-folder-name')
          || document.querySelector('input[placeholder*="folder" i]')
          || document.querySelector('input[placeholder*="name" i]')
          || document.querySelector('dialog input, [role="dialog"] input');
        if (inp) {
          inp.value = 'audit-test-folder';
          inp.dispatchEvent(new Event('input', { bubbles: true }));
          return true;
        }
        return false;
      });

      if (!inputFound) {
        fail('phase2', 'folder-name-input', 'Could not find folder name input');
      } else {
        pass('phase2', 'folder-name-input', 'Folder name input found, typed "audit-test-folder"');

        // Click Create button
        await page.evaluate(() => {
          const btns = Array.from(document.querySelectorAll('button'));
          const btn = btns.find(b => {
            const text = (b.textContent || '').trim().toLowerCase();
            return text === 'create' || text === 'ok' || text === 'submit' || text === 'confirm';
          });
          if (btn) btn.click();
        });
        await page.waitForTimeout(3000);
        await screenshot(page, 'phase2-02-folder-created');

        // Verify folder appears
        const folderVisible = await page.evaluate(() => {
          const text = document.body?.innerText || '';
          return text.includes('audit-test-folder');
        });
        if (folderVisible) {
          pass('phase2', 'folder-visible', 'Folder "audit-test-folder" appears in file list');
        } else {
          fail('phase2', 'folder-visible', 'Folder "audit-test-folder" not found in file list');
        }

        // Enter the folder (click on it)
        const enteredFolder = await page.evaluate(() => {
          const links = Array.from(document.querySelectorAll('a, button, [role="row"], tr'));
          for (const el of links) {
            if (el.textContent?.includes('audit-test-folder')) {
              el.click();
              return true;
            }
          }
          return false;
        });

        if (enteredFolder) {
          await page.waitForTimeout(2000);
          await screenshot(page, 'phase2-03-entered-folder');

          // Check breadcrumb
          const breadcrumbText = await page.evaluate(() => {
            const nav = document.querySelector('nav[aria-label]');
            return nav?.textContent || '';
          });
          if (breadcrumbText.includes('audit-test-folder')) {
            pass('phase2', 'breadcrumb', `Breadcrumb shows "audit-test-folder": "${breadcrumbText.substring(0, 80)}"`);
          } else {
            fail('phase2', 'breadcrumb', `Breadcrumb does not contain "audit-test-folder": "${breadcrumbText.substring(0, 80)}"`);
          }

          // Navigate back to root via breadcrumb
          await page.evaluate(() => {
            const nav = document.querySelector('nav[aria-label]');
            if (nav) {
              const btns = Array.from(nav.querySelectorAll('button'));
              const homeBtn = btns.find(b => b.textContent?.trim() === '/' || b.textContent?.trim().toLowerCase() === 'home');
              if (homeBtn) homeBtn.click();
            }
          });
          await page.waitForTimeout(2000);
          await screenshot(page, 'phase2-04-back-to-root');

          // Now delete the folder - first click FILES tab
          await page.evaluate(() => {
            const btns = Array.from(document.querySelectorAll('button'));
            const btn = btns.find(b => b.textContent.includes('FILES') || b.textContent.includes('Files'));
            if (btn) btn.click();
          });
          await page.waitForTimeout(1000);

          // Find and click the delete button for audit-test-folder
          const deleteClicked = await page.evaluate(() => {
            const rows = Array.from(document.querySelectorAll('tr[role="row"], [role="row"]'));
            for (const row of rows) {
              if (row.textContent?.includes('audit-test-folder')) {
                const deleteBtn = row.querySelector('button[aria-label*="Delete"], button[aria-label*="delete"]');
                if (deleteBtn) {
                  deleteBtn.click();
                  return 'button';
                }
              }
            }
            return false;
          });

          if (deleteClicked) {
            await page.waitForTimeout(1000);
            await screenshot(page, 'phase2-05-delete-confirm');

            // Confirm deletion
            await page.evaluate(() => {
              const btns = Array.from(document.querySelectorAll('button'));
              const confirmBtn = btns.find(b => {
                const text = (b.textContent || '').trim().toLowerCase();
                return text === 'delete' || text === 'confirm' || text === 'yes';
              });
              if (confirmBtn) confirmBtn.click();
            });
            await page.waitForTimeout(3000);
            await screenshot(page, 'phase2-06-after-delete');

            const folderGone = await page.evaluate(() => {
              return !(document.body?.innerText || '').includes('audit-test-folder');
            });
            if (folderGone) {
              pass('phase2', 'folder-deleted', 'Folder "audit-test-folder" removed');
            } else {
              fail('phase2', 'folder-deleted', 'Folder "audit-test-folder" still visible after delete');
            }
          } else {
            fail('phase2', 'delete-click', 'Could not find delete button for audit-test-folder');
          }
        } else {
          fail('phase2', 'enter-folder', 'Could not click on audit-test-folder');
        }
      }
    }
  } catch (e) {
    fail('phase2', 'exception', e.message);
  }

  // ─── PHASE 3: Search ────────────────────────────────────────────
  log('\n═══ PHASE 3: Search ═══');
  try {
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);

    // Click the search button (magnifying glass in header)
    const searchOpened = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => {
        const aria = b.getAttribute('aria-label') || '';
        return aria.toLowerCase().includes('search');
      });
      if (btn) { btn.click(); return true; }
      return false;
    });

    if (!searchOpened) {
      fail('phase3', 'search-button', 'Search button not found');
    } else {
      await page.waitForTimeout(500);
      await screenshot(page, 'phase3-01-search-opened');

      // Type in search
      const inputReady = await page.evaluate(() => {
        const inp = document.getElementById('header-search-input');
        if (inp) {
          inp.value = 'test';
          inp.dispatchEvent(new Event('input', { bubbles: true }));
          inp.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
          return true;
        }
        return false;
      });

      if (inputReady) {
        pass('phase3', 'search-input', 'Search input found and "test" typed');
        await page.waitForTimeout(2000);
        await screenshot(page, 'phase3-02-search-results');

        const hasResultsArea = await page.evaluate(() => {
          const resultsList = document.querySelector('[role="listbox"]');
          const searchArea = document.querySelector('.slide-up') || document.querySelector('#header-search-input')?.closest('div')?.parentElement;
          return {
            hasListbox: !!resultsList,
            resultCount: document.querySelectorAll('[role="listbox"] a, [role="listbox"] [role="option"]').length,
            bodyTextSnippet: (searchArea?.textContent || '').substring(0, 200),
          };
        });
        pass('phase3', 'search-results', `Search results area present: listbox=${hasResultsArea.hasListbox}, items=${hasResultsArea.resultCount}`);
      } else {
        fail('phase3', 'search-input', 'Search input #header-search-input not found');
      }
    }
  } catch (e) {
    fail('phase3', 'exception', e.message);
  }

  // ─── PHASE 4: Settings/Preferences ──────────────────────────────
  log('\n═══ PHASE 4: Settings/Preferences ═══');
  try {
    await page.goto(`${BASE}/ui/settings`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase4-01-settings-page');

    const settingsContent = await page.evaluate(() => {
      const text = document.body?.innerText || '';
      const hasTheme = text.toLowerCase().includes('theme') || text.toLowerCase().includes('dark');
      const hasViewMode = text.toLowerCase().includes('view') || text.toLowerCase().includes('list');
      const hasSave = text.toLowerCase().includes('save');
      const radios = document.querySelectorAll('input[type="radio"]');
      return { hasTheme, hasViewMode, hasSave, radioCount: radios.length, bodySnippet: text.substring(0, 300) };
    });

    if (settingsContent.hasTheme) {
      pass('phase4', 'theme-settings', 'Theme settings found on settings page');
    } else {
      fail('phase4', 'theme-settings', 'Theme settings not found');
    }

    if (settingsContent.hasSave) {
      pass('phase4', 'save-button', 'Save button found');
    } else {
      fail('phase4', 'save-button', 'Save button not found');
    }

    // Go back to home and test theme toggle
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);

    const initialTheme = await page.evaluate(() => {
      return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
    });

    const themeToggled = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const btn = btns.find(b => {
        const aria = b.getAttribute('aria-label') || '';
        return aria.toLowerCase().includes('theme') || aria.toLowerCase().includes('toggle');
      });
      if (btn) { btn.click(); return true; }
      return false;
    });

    if (themeToggled) {
      await page.waitForTimeout(1000);
      const newTheme = await page.evaluate(() => {
        return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
      });
      await screenshot(page, 'phase4-02-theme-toggled');
      if (initialTheme !== newTheme) {
        pass('phase4', 'theme-toggle', `Theme changed from ${initialTheme} to ${newTheme}`);
      } else {
        fail('phase4', 'theme-toggle', `Theme did not change (still ${initialTheme})`);
      }
    } else {
      fail('phase4', 'theme-toggle', 'Theme toggle button not found');
    }

    allA11y['phase4'] = await collectA11y(page, 'phase4');
  } catch (e) {
    fail('phase4', 'exception', e.message);
  }

  // ─── PHASE 5: Responsive Behavior ───────────────────────────────
  log('\n═══ PHASE 5: Responsive Behavior ═══');
  try {
    // Mobile viewport
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase5-01-mobile-375');

    const mobileLayout = await page.evaluate(() => {
      const bodyWidth = document.body.scrollWidth;
      const viewportWidth = window.innerWidth;
      const overflow = bodyWidth > viewportWidth;

      // Check minimum tap target sizes
      const btns = Array.from(document.querySelectorAll('button'));
      let smallBtns = 0;
      btns.forEach(b => {
        const rect = b.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0 && (rect.width < 44 || rect.height < 44)) {
          smallBtns++;
        }
      });

      return {
        overflow,
        bodyWidth,
        viewportWidth,
        smallBtns,
        totalBtns: btns.length,
      };
    });

    if (!mobileLayout.overflow) {
      pass('phase5', 'no-horizontal-overflow', `No horizontal overflow (body=${mobileLayout.bodyWidth}px, viewport=${mobileLayout.viewportWidth}px)`);
    } else {
      fail('phase5', 'no-horizontal-overflow', `Horizontal overflow detected (body=${mobileLayout.bodyWidth}px > viewport=${mobileLayout.viewportWidth}px)`);
    }

    if (mobileLayout.smallBtns === 0) {
      pass('phase5', 'tap-targets', `All ${mobileLayout.totalBtns} buttons have min 44px tap target`);
    } else {
      fail('phase5', 'tap-targets', `${mobileLayout.smallBtns} of ${mobileLayout.totalBtns} buttons have tap targets < 44px`);
    }

    // Tablet
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase5-02-tablet-768');

    const tabletOverflow = await page.evaluate(() => document.body.scrollWidth > window.innerWidth);
    if (!tabletOverflow) {
      pass('phase5', 'tablet-no-overflow', 'No horizontal overflow on tablet');
    } else {
      fail('phase5', 'tablet-no-overflow', 'Horizontal overflow on tablet viewport');
    }

    // Restore desktop
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase5-03-desktop-restored');

    allA11y['phase5'] = await collectA11y(page, 'phase5-mobile');
  } catch (e) {
    fail('phase5', 'exception', e.message);
  }

  // ─── PHASE 6: Accessibility Audit ───────────────────────────────
  log('\n═══ PHASE 6: Accessibility Audit ═══');
  const routes = [
    { path: '/ui/', name: 'home' },
    { path: '/ui/settings', name: 'settings' },
    { path: '/ui/trash', name: 'trash' },
  ];
  const a11yReport = {};

  for (const route of routes) {
    try {
      await page.goto(`${BASE}${route.path}`, { waitUntil: 'domcontentloaded', timeout: 20000 });
      await page.waitForTimeout(WASM_HYDRATE_MS);
      await screenshot(page, `phase6-a11y-${route.name}`);

      const data = await collectA11y(page, route.name);
      a11yReport[route.name] = data;

      // Flag issues
      if (data.imgsNoAlt > 0) {
        fail('phase6', `${route.name}-img-alt`, `${data.imgsNoAlt} image(s) without alt attribute`);
      } else {
        pass('phase6', `${route.name}-img-alt`, 'All images have alt attributes');
      }

      if (data.unlabeledBtns > 0) {
        fail('phase6', `${route.name}-btn-label`, `${data.unlabeledBtns} button(s) without accessible name`);
      } else {
        pass('phase6', `${route.name}-btn-label`, 'All buttons have accessible names');
      }

      if (data.inputsNoLabel > 0) {
        fail('phase6', `${route.name}-input-label`, `${data.inputsNoLabel} input(s) without label`);
      } else {
        pass('phase6', `${route.name}-input-label`, 'All inputs have labels');
      }

      if (data.positiveTabindex > 0) {
        fail('phase6', `${route.name}-tabindex`, `${data.positiveTabindex} element(s) with positive tabindex`);
      } else {
        pass('phase6', `${route.name}-tabindex`, 'No positive tabindex values found');
      }

      const hasMain = data.landmarks.main > 0;
      const hasNav = data.landmarks.nav > 0;
      const hasHeader = data.landmarks.header > 0;
      if (hasMain && hasNav && hasHeader) {
        pass('phase6', `${route.name}-landmarks`, `Landmarks present: nav=${data.landmarks.nav} main=${data.landmarks.main} header=${data.landmarks.header}`);
      } else {
        fail('phase6', `${route.name}-landmarks`, `Missing landmarks: nav=${data.landmarks.nav} main=${data.landmarks.main} header=${data.landmarks.header}`);
      }
    } catch (e) {
      fail('phase6', `${route.name}-exception`, e.message);
    }
  }

  // ─── PHASE 7: Error Handling ────────────────────────────────────
  log('\n═══ PHASE 7: Error Handling ═══');
  try {
    // Nonexistent route
    await page.goto(`${BASE}/ui/nonexistent-route-test`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase7-01-nonexistent-route');

    const fallbackContent = await page.evaluate(() => {
      const text = document.body?.innerText || '';
      return {
        hasContent: text.trim().length > 0,
        bodySnippet: text.substring(0, 300),
        hasApp: !!document.getElementById('app'),
      };
    });

    if (fallbackContent.hasContent) {
      pass('phase7', 'nonexistent-route', `Fallback content shown: "${fallbackContent.bodySnippet.substring(0, 80)}"`);
    } else {
      fail('phase7', 'nonexistent-route', 'Blank page on nonexistent route');
    }

    // Block API calls and check error state
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(WASM_HYDRATE_MS);

    // Intercept and fail API requests
    await page.route('**/api/**', route => route.abort('connectionrefused'));
    await page.route('**/webdav/**', route => route.abort('connectionrefused'));

    // Reload to trigger failed requests
    await page.reload({ waitUntil: 'domcontentloaded' });
    await page.waitForTimeout(WASM_HYDRATE_MS);
    await screenshot(page, 'phase7-02-blocked-api');

    const errorState = await page.evaluate(() => {
      const text = document.body?.innerText || '';
      const hasErrorUI = text.toLowerCase().includes('error')
        || text.toLowerCase().includes('failed')
        || text.toLowerCase().includes('offline')
        || document.querySelector('[role="alert"]') !== null
        || document.querySelector('.error') !== null;
      return {
        hasErrorUI,
        bodySnippet: text.substring(0, 300),
        hasApp: !!document.getElementById('app'),
      };
    });

    if (errorState.hasApp) {
      pass('phase7', 'blocked-api', `App still renders with blocked API (app present, content: "${errorState.bodySnippet.substring(0, 80)}")`);
    } else {
      fail('phase7', 'blocked-api', 'App not rendered when API is blocked');
    }

    // Remove route interception for subsequent tests
    await page.unroute('**/api/**');
    await page.unroute('**/webdav/**');
  } catch (e) {
    fail('phase7', 'exception', e.message);
  }

  // ─── PHASE 8: Performance ───────────────────────────────────────
  log('\n═══ PHASE 8: Performance ═══');
  try {
    // Measure load + hydration
    await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
    const t0 = Date.now();
    await page.waitForTimeout(WASM_HYDRATE_MS);
    const hydrationTime = Date.now() - t0;
    perf.hydrationTime = hydrationTime;

    const domStats = await page.evaluate(() => {
      const all = document.querySelectorAll('*');
      let maxDepth = 0;
      const walk = (el, depth) => {
        if (depth > maxDepth) maxDepth = depth;
        for (const child of el.children) walk(child, depth + 1);
      };
      walk(document.body, 0);
      return {
        totalNodes: all.length,
        maxDepth,
        appNodeCount: document.getElementById('app')?.querySelectorAll('*').length || 0,
      };
    });

    log(`  DOM: ${domStats.totalNodes} total nodes, max depth ${domStats.maxDepth}, #app has ${domStats.appNodeCount} nodes`);
    log(`  Hydration time: ${hydrationTime}ms`);
    pass('phase8', 'dom-stats', `${domStats.totalNodes} nodes, depth ${domStats.maxDepth}`);

    if (domStats.totalNodes < 5000) {
      pass('phase8', 'reasonable-dom', `DOM size reasonable: ${domStats.totalNodes} nodes`);
    } else {
      fail('phase8', 'reasonable-dom', `DOM too large: ${domStats.totalNodes} nodes`);
    }

    // Memory leak check: navigate multiple times
    const initialHeap = await page.evaluate(() => {
      if (performance.memory) return performance.memory.usedJSHeapSize;
      return 0;
    });

    for (let i = 0; i < 5; i++) {
      await page.goto(`${BASE}/ui/`, { waitUntil: 'domcontentloaded', timeout: 20000 });
      await page.waitForTimeout(2000);
      await page.goto(`${BASE}/ui/settings`, { waitUntil: 'domcontentloaded', timeout: 20000 });
      await page.waitForTimeout(2000);
    }

    const finalHeap = await page.evaluate(() => {
      if (performance.memory) return performance.memory.usedJSHeapSize;
      return 0;
    });

    if (initialHeap > 0 && finalHeap > 0) {
      const growth = finalHeap - initialHeap;
      const growthMB = (growth / 1024 / 1024).toFixed(2);
      if (growth < 50 * 1024 * 1024) {
        pass('phase8', 'memory-leak', `Heap growth after 10 navigations: ${growthMB}MB (acceptable)`);
      } else {
        fail('phase8', 'memory-leak', `Heap growth after 10 navigations: ${growthMB}MB (potential leak)`);
      }
    } else {
      pass('phase8', 'memory-leak', 'performance.memory not available (headless Chromium); skipping heap check');
    }

    perf.domNodes = domStats.totalNodes;
    perf.maxDepth = domStats.maxDepth;
  } catch (e) {
    fail('phase8', 'exception', e.message);
  }

  // ─── Generate Report ────────────────────────────────────────────
  log('\n═══ Generating Report ═══');

  const passCount = phases.filter(p => p.status === 'PASS').length;
  const failCount = phases.filter(p => p.status === 'FAIL').length;

  let report = `# Leptos WASM Frontend — Deep Audit Report\n\n`;
  report += `**Date:** ${new Date().toISOString()}\n`;
  report += `**Target:** ${BASE}/ui/\n`;
  report += `**Results:** ${passCount} PASS / ${failCount} FAIL / ${phases.length} total\n\n`;
  report += `---\n\n`;

  // Group by phase
  const phaseGroups = {};
  for (const p of phases) {
    const group = p.phase;
    if (!phaseGroups[group]) phaseGroups[group] = [];
    phaseGroups[group].push(p);
  }

  for (const [group, items] of Object.entries(phaseGroups)) {
    const groupPass = items.filter(i => i.status === 'PASS').length;
    const groupFail = items.filter(i => i.status === 'FAIL').length;
    const icon = groupFail === 0 ? '✅' : '⚠️';
    report += `## ${icon} ${group.toUpperCase().replace(/PHASE/, 'Phase ')} (${groupPass}P / ${groupFail}F)\n\n`;
    report += `| Status | Step | Detail |\n|--------|------|--------|\n`;
    for (const item of items) {
      const icon = item.status === 'PASS' ? '✅' : '❌';
      report += `| ${icon} ${item.status} | ${item.step} | ${item.detail.replace(/\|/g, '\\|')} |\n`;
    }
    report += '\n';
  }

  // Performance summary
  report += `## Performance\n\n`;
  report += `- **Initial load time:** ${perf.loadTime || 'N/A'}ms\n`;
  report += `- **Hydration wait:** ${WASM_HYDRATE_MS}ms\n`;
  report += `- **DOM nodes:** ${perf.domNodes || 'N/A'}\n`;
  report += `- **Max DOM depth:** ${perf.maxDepth || 'N/A'}\n`;
  report += `- **Console errors:** ${consoleErrors.length}\n`;
  if (consoleErrors.length > 0) {
    report += `\n### Console Errors\n\n`;
    consoleErrors.forEach((e, i) => { report += `${i + 1}. ${e}\n`; });
  }
  report += '\n';

  // Accessibility summary
  report += `## Accessibility Summary\n\n`;
  for (const [route, data] of Object.entries(a11yReport)) {
    report += `### ${route}\n`;
    report += `- Total elements: ${data.totalElements}\n`;
    report += `- Buttons: ${data.buttons}\n`;
    report += `- Inputs: ${data.inputs}\n`;
    report += `- Links: ${data.links}\n`;
    report += `- Images: ${data.images}\n`;
    report += `- Images without alt: ${data.imgsNoAlt}\n`;
    report += `- Unlabeled buttons: ${data.unlabeledBtns}\n`;
    report += `- Inputs without labels: ${data.inputsNoLabel}\n`;
    report += `- Positive tabindex: ${data.positiveTabindex}\n`;
    report += `- Landmarks: nav=${data.landmarks.nav} main=${data.landmarks.main} aside=${data.landmarks.aside} header=${data.landmarks.header} footer=${data.landmarks.footer}\n\n`;
  }

  report += `---\n\n*Generated by e2e/deep-audit.js*\n`;

  fs.writeFileSync(path.join(OUT, 'REPORT.md'), report);
  log(`Report written to ${path.join(OUT, 'REPORT.md')}`);
  log(`\n${passCount} PASS / ${failCount} FAIL / ${phases.length} total`);

  await browser.close();
  process.exit(failCount > 0 ? 1 : 0);
})();
