# UI/UX Feature Research — File Managers & Productivity Apps

## 1. Nautilus (GNOME Files)

| Category | Features |
|----------|----------|
| **Navigation** | Breadcrumb path bar, sidebar bookmarks, keyboard-driven navigation, recent locations |
| **File Management** | Create/move/copy/delete, batch operations, trash support, symbolic links |
| **Search & Filtering** | Real-time search bar, type-ahead search, full-text search |
| **Sorting** | Sort by name/date/size/type, ascending/descending toggle |
| **View Modes** | Icon Grid, Icon List, Tree List (3 views) |
| **Drag & Drop** | Native drag-and-drop between folders, external app drops |
| **Keyboard Shortcuts** | Standard GNOME shortcuts (Ctrl+C/X/V, Delete, F2 rename, Ctrl+L location bar) |
| **Context Menus** | Right-click context menu with actions, Open With, Properties |
| **Theming** | Follows GNOME system theme (Adwaita), dark/light mode |
| **Responsive** | Adaptive layout for window resizing, touch-friendly |
| **Accessibility** | High contrast, screen reader support (AT-SPI), keyboard navigation |
| **Offline** | Full offline (local filesystem) |
| **Collaboration** | None (single-user file manager) |
| **Media Handling** | Thumbnail generation, image preview, video preview |
| **Performance** | Lazy loading of thumbnails, async file operations |
| **Error Handling** | Dialog for file conflicts (overwrite/skip/rename), permission errors |
| **Onboarding** | GNOME Help integration, tooltips |
| **Customization** | Sidebar configuration, plugin/script extensions |

---

## 2. Dolphin (KDE)

| Category | Features |
|----------|----------|
| **Navigation** | Breadcrumbs, path bar, sidebar panels, tabs, split view (dual pane) |
| **File Management** | Full CRUD, batch operations, copy queue, symbolic links, metadata |
| **Search & Filtering** | Baloo search integration, type-ahead, filter bar |
| **Sorting** | Sort by name/date/size/type/permissions, customizable columns |
| **View Modes** | Icons, Details (list), Compact (tree), Thumbnail view |
| **Drag & Drop** | Drag between split panes, drag to terminal, drag to external apps |
| **Keyboard Shortcuts** | Extensive shortcuts, customizable, F2 rename, F3 viewer, F4 terminal |
| **Context Menus** | Rich right-click menu, custom actions, service menus (KIO) |
| **Theming** | KDE Plasma themes, custom icons, color schemes, configurable toolbar |
| **Responsive** | Resizable panels, collapsible sidebar |
| **Accessibility** | Keyboard navigation, screen reader support, high contrast |
| **Offline** | Full offline + network filesystems (SMB, NFS, SSH via KIO) |
| **Collaboration** | Samba file sharing plugin, network browsing |
| **Media Handling** | Thumbnail generation (FFmpeg plugin), file preview panel, embedded terminal |
| **Performance** | Async I/O, KIO slaves for non-blocking operations |
| **Error Handling** | Conflict resolution dialogs, skip/retry/overwrite options |
| **Onboarding** | KDE Handbook, tooltips, first-run wizard |
| **Customization** | Panels, toolbars, view properties, plugins (git, Dropbox, Nextcloud, GDrive) |

---

## 3. Finder (macOS)

| Category | Features |
|----------|----------|
| **Navigation** | Sidebar favorites, path bar, go-to-folder, recent, tags |
| **File Management** | Full CRUD, Aliases, Packages, Bundle management |
| **Search & Filtering** | Spotlight integration, search filters (kind/date/name), saved searches |
| **Sorting** | Sort by name/date/size/kind/date added, grouped sorting |
| **View Modes** | Icon, List, Column, Gallery |
| **Drag & Drop** | Drag between Finder windows, Stacks, Dock, external apps, Spring-loading |
| **Keyboard Shortcuts** | Cmd+C/X/V, Space quick look, Cmd+Shift+N new folder, arrow navigation |
| **Context Menus** | Quick Actions, Shortcuts integration, services, share sheet |
| **Theming** | macOS system theme, dark/light mode, accent colors |
| **Responsive** | Adaptive columns, resizable thumbnails |
| **Accessibility** | VoiceOver, full keyboard nav, Dynamic Type, Voice Control |
| **Offline** | Full offline + iCloud Drive |
| **Collaboration** | iCloud sharing, AirDrop, SharePlay |
| **Media Handling** | Quick Look (Space), thumbnail previews, image/video playback |
| **Performance** | Virtual file system (FUSE for macOS), lazy loading |
| **Error Handling** | Native macOS dialogs, merge/replace/skip options |
| **Onboarding** | macOS Tips app, tooltips, What's New screens |
| **Customization** | Toolbar customization, sidebar tags, view options per folder |

---

## 4. Total Commander

| Category | Features |
|----------|----------|
| **Navigation** | Dual-pane side-by-side, tabs per pane, history, bookmarks, drive bar |
| **File Management** | Full CRUD, batch operations, multi-rename tool, sync directories |
| **Search & Filtering** | Enhanced search with regex, full content search, duplicate finder |
| **Sorting** | Sort by any column, custom sort orders |
| **View Modes** | Full, Custom columns, Thumbnails, Tree |
| **Drag & Drop** | Drag between panes, to external apps, queue operations |
| **Keyboard Shortcuts** | Extensive (F3 view, F4 edit, F5 copy, F7 mkdir, F8 delete), customizable |
| **Context Menus** | Configurable context menu, 64-bit context menu support |
| **Theming** | Custom color schemes, icon packs, font selection |
| **Responsive** | Configurable pane widths, font scaling |
| **Accessibility** | Keyboard-only operation, screen reader support |
| **Offline** | Full offline + FTP/FTPS client built-in |
| **Collaboration** | Network browsing (FTP, SSH, cloud) |
| **Media Handling** | Quick View panel (F3) for images/video, thumbnail view |
| **Performance** | Background operations, partial branch view (Ctrl+Shift+B) |
| **Error Handling** | Enhanced overwrite dialog, file comparison before copy |
| **Onboarding** | Built-in help, image gallery of features, FAQ |
| **Customization** | Button bar, custom columns, WCX/WDX/WFX/WLX plugin system |

---

## 5. Double Commander

| Category | Features |
|----------|----------|
| **Navigation** | Dual-pane, tabs, bookmarks, history, drive bar |
| **File Management** | Full CRUD, batch operations, multi-rename tool |
| **Search & Filtering** | Extended search, full-text search in any files |
| **Sorting** | Sort by columns, custom column views |
| **View Modes** | Full, Custom columns, Thumbnails |
| **Drag & Drop** | Drag between panes |
| **Keyboard Shortcuts** | Total Commander compatible (F3 viewer, F4 editor, etc.), customizable |
| **Context Menus** | Configurable context menu |
| **Theming** | Customizable colors, fonts, icons |
| **Responsive** | Configurable layout |
| **Accessibility** | Keyboard navigation |
| **Offline** | Full offline + FTP/FTPS/SSH/SCP/SFTP support |
| **Collaboration** | Network protocols (FTP, SSH) |
| **Media Handling** | Built-in file viewer (F3) - hex/binary/text, internal text editor (F4) |
| **Performance** | Background operations for most file operations |
| **Error Handling** | File operation logging, conflict resolution |
| **Onboarding** | Documentation, wiki |
| **Customization** | Button bar, TC plugin support (WCX/WDX/WFX/WLX), configurable columns |

---

## 6. Notion

| Category | Features |
|----------|----------|
| **Navigation** | Sidebar with nested pages, breadcrumbs, favorites, search (Cmd+P), recent pages |
| **Item Management** | Pages, databases, blocks (text, images, tables, toggles, code, embeds) |
| **Search & Filtering** | Global search, database filters, saved filters, full-text search |
| **Sorting** | Database sorting by any property, multi-sort |
| **View Modes** | Table, Board (Kanban), Timeline, Calendar, List, Gallery |
| **Drag & Drop** | Block reordering, database row reordering, sidebar page reordering |
| **Keyboard Shortcuts** | Markdown shortcuts, /-commands, Cmd+K for actions, extensive keybindings |
| **Context Menus** | Block-level menus, database row menus, page actions |
| **Theming** | Light/dark mode, custom page covers/icons, limited color palettes |
| **Responsive** | Web + desktop + mobile apps, responsive layout |
| **Accessibility** | Keyboard navigation, screen reader support, ARIA labels |
| **Offline** | Limited offline (cached pages), sync on reconnect |
| **Collaboration** | Real-time co-editing, comments, mentions, page sharing, workspace permissions |
| **Media Handling** | Image/video/file embeds, bookmarks, code blocks |
| **Performance** | Incremental loading, virtual scrolling for large databases |
| **Error Handling** | Auto-save, version history (30 days), conflict resolution |
| **Onboarding** | Product tours, templates, help center, Notion Academy |
| **Customization** | Templates, database properties, formulas, relations, rollups, AI agents |

---

## 7. Obsidian

| Category | Features |
|----------|----------|
| **Navigation** | File explorer sidebar, tabs, breadcrumbs, quick switcher (Cmd+O), backlinks panel |
| **Item Management** | Markdown files, folders, attachments, vault structure |
| **Search & Filtering** | Full-text search, regex, tag filtering, property-based search |
| **Sorting** | Sort by name/date/size, custom order |
| **View Modes** | File explorer, outline, graph view, backlinks, Canvas (infinite whiteboard) |
| **Drag & Drop** | File reordering, link creation via drag, Canvas node arrangement |
| **Keyboard Shortcuts** | Vim/standard modes, command palette (Cmd+P), customizable hotkeys |
| **Context Menus** | File context menus, link context menus, block-level actions |
| **Theming** | CSS-based themes, dark/light mode, community themes (thousands) |
| **Responsive** | Desktop + mobile apps, responsive layout |
| **Accessibility** | Keyboard navigation, screen reader support, font scaling |
| **Offline** | Full offline (local files), no server dependency |
| **Collaboration** | Shared vaults via Sync, comments (limited), Publish for web |
| **Media Handling** | Image embeds, PDF viewing, audio/video playback, excalidraw |
| **Performance** | Fast local rendering, lazy loading for large vaults |
| **Error Handling** | Local git-friendly files, recovery modes, corrupted file detection |
| **Onboarding** | Help vault, community plugins guide, tutorials |
| **Customization** | 2000+ community plugins, CSS snippets, hotkeys, workspace layouts, Canvas |

---

## 8. Linear

| Category | Features |
|----------|----------|
| **Navigation** | Sidebar (Inbox, My Issues, Projects, Teams), Cmd+K command palette, breadcrumbs |
| **Item Management** | Issues, Projects, Cycles, Initiatives, Documents, Labels, Relationships |
| **Search & Filtering** | Powerful filter bar (status/priority/assignee/label/team), keyboard-driven |
| **Sorting** | Sort by priority/created/updated/assignee, custom views |
| **View Modes** | List, Board (Kanban), Timeline, Custom views |
| **Drag & Drop** | Issue reordering, status changes via drag, project assignment |
| **Keyboard Shortcuts** | Extensive (j/k navigation, c new issue, s status, e edit, / search) |
| **Context Menus** | Right-click issue menus, bulk actions |
| **Theming** | Dark/light mode, minimal design, system theme sync |
| **Responsive** | Web + desktop + mobile apps |
| **Accessibility** | Full keyboard navigation, screen reader support, ARIA |
| **Offline** | Limited offline (cached state), sync on reconnect |
| **Collaboration** | Real-time updates, comments, mentions, team workspaces, agent workflows |
| **Media Handling** | File attachments, image previews, code snippets |
| **Performance** | Instant UI updates, optimistic mutations, background sync |
| **Error Handling** | Auto-retry, offline queue, error boundaries |
| **Onboarding** | Product tours, Linear Method documentation, keyboard shortcuts guide |
| **Customization** | Custom views, saved filters, templates, automation rules, integrations (GitHub/GitLab) |

---

## 9. Raycast

| Category | Features |
|----------|----------|
| **Navigation** | Global hotkey (Cmd+Space), fuzzy search, command palette pattern |
| **Item Management** | Extensions, snippets, quicklinks, clipboard history, notes |
| **Search & Filtering** | Instant fuzzy search, category filtering, extension search |
| **Sorting** | Recent commands, pinned favorites, usage-based ranking |
| **View Modes** | List view with details panel, single-command views |
| **Drag & Drop** | Limited (clipboard history items) |
| **Keyboard Shortcuts** | Entirely keyboard-first, customizable hotkeys, aliases, snippets with keywords |
| **Context Menus** | Extension-specific actions, quicklinks |
| **Theming** | Dark/light mode, system theme, accent colors, transparency effects |
| **Responsive** | Window size adjustable, compact/expanded modes |
| **Accessibility** | Full keyboard operation, VoiceOver support |
| **Offline** | Core features offline, extensions may need network |
| **Collaboration** | None (personal productivity tool) |
| **Media Handling** | Emoji picker, screenshot search, file search |
| **Performance** | Sub-100ms launch, instant search, 99.8% crash-free rate |
| **Error Handling** | Extension error isolation, graceful fallbacks |
| **Onboarding** | First-run tutorial, extension recommendations, tips |
| **Customization** | Extension store (thousands), custom snippets, quicklinks, script commands, hotkeys, window management |

---

## 10. Craft

| Category | Features |
|----------|----------|
| **Navigation** | Sidebar (Spaces, Folders, Tags), backlinks, quick search, daily notes |
| **Item Management** | Documents, blocks (text, tasks, images, tables), folders, spaces, collections |
| **Search & Filtering** | Full-text search, tag filtering, folder navigation |
| **Sorting** | Sort by date/name, manual ordering |
| **View Modes** | Document view, whiteboard (Canvas), calendar view, task list |
| **Drag & Drop** | Block reordering, folder organization, document nesting |
| **Keyboard Shortcuts** | Markdown shortcuts, Cmd+K command palette, task shortcuts |
| **Context Menus** | Block-level menus, document actions, share options |
| **Theming** | Multiple themes, dark/light mode, paper textures, custom backgrounds |
| **Responsive** | Native Mac/iOS/iPad/Web/Android/Windows apps, responsive layout |
| **Accessibility** | VoiceOver support, keyboard navigation, Dynamic Type |
| **Offline** | Full offline with local storage, sync on reconnect |
| **Collaboration** | Real-time co-editing, sharing links, collaborative workspaces |
| **Media Handling** | Image/video embeds, file attachments, whiteboard sketches |
| **Performance** | Native rendering, fast sync, optimized for Apple silicon |
| **Error Handling** | Auto-save, version history, conflict resolution |
| **Onboarding** | Getting started guide, templates, community templates, tutorials |
| **Customization** | Templates, spaces, folders, tags, MCP connections, API integrations, AI writing |

---

## 11. Adobe Lightroom

| Category | Features |
|----------|----------|
| **Navigation** | Module switching (Library/Develop/Map/Book/Slideshow/Print/Web), folders, collections |
| **Item Management** | Photos, albums, smart collections, keywords, metadata |
| **Search & Filtering** | Advanced filtering (camera/lens/date/rating/flag/keyword), text search, attribute filters |
| **Sorting** | Sort by capture date, import date, filename, rating, custom order |
| **View Modes** | Grid, Loupe, Compare, Survey, People, Map, Filmstrip |
| **Drag & Drop** | Drag to collections, drag to reorder, drag between folders |
| **Keyboard Shortcuts** | Extensive module-specific shortcuts (G grid, E loupe, D develop) |
| **Context Menus** | Right-click photo menus, export options, metadata actions |
| **Theming** | Dark UI (standard), customizable workspace |
| **Responsive** | Desktop + mobile apps, cloud sync |
| **Accessibility** | Keyboard navigation, screen reader support (limited) |
| **Offline** | Full offline for local catalog, cloud sync when online |
| **Collaboration** | Shared albums, Lightroom web galleries, comments |
| **Media Handling** | RAW processing, EXIF/IPTC metadata, GPS, face detection, batch editing |
| **Performance** | GPU-accelerated rendering, smart previews, lazy loading |
| **Error Handling** | Catalog backup, import error reporting, missing file detection |
| **Onboarding** | In-app tutorials, guided edits, Adobe Learn |
| **Customization** | Presets, develop settings, export presets, keyboard shortcut customization, plugin support |

---

## 12. digiKam

| Category | Features |
|----------|----------|
| **Navigation** | Album tree, tag tree, labels, date navigator, timeline, geolocation map |
| **Item Management** | Albums, tags, labels, ratings, metadata, face tags |
| **Search & Filtering** | Advanced search (tags/labels/dates/geolocation/camera/lens), similar image search |
| **Sorting** | Sort by date/rating/metadata, calendar view |
| **View Modes** | Thumbnails, table, preview, map view, calendar view, face management |
| **Drag & Drop** | Drag to albums, tags, external applications |
| **Keyboard Shortcuts** | Extensive shortcuts for navigation, editing, tagging |
| **Context Menus** | Right-click menus with batch operations, export, edit |
| **Theming** | KDE/Qt themes, icon themes, customizable interface |
| **Responsive** | Desktop application, resizable panels |
| **Accessibility** | Keyboard navigation, screen reader support |
| **Offline** | Full offline (local database + files) |
| **Collaboration** | Export to social media, web galleries, email sharing |
| **Media Handling** | RAW processing, face recognition, AI tagging, color histograms, batch processing, video support |
| **Performance** | Handles 100,000+ images, database indexing, lazy loading |
| **Error Handling** | Database backup/restore, corruption detection, import error reporting |
| **Onboarding** | Documentation, FAQ, tutorials |
| **Customization** | External tool integration, metadata templates, custom rules, plugin system |

---

## 13. Plex

| Category | Features |
|----------|----------|
| **Navigation** | Library sidebar, home screen, search, discover section, watchlist |
| **Item Management** | Libraries (movies, TV, music, photos), collections, playlists, watchlist |
| **Search & Filtering** | Global search, genre filtering, year filtering, rating filtering |
| **Sorting** | Sort by title/date/rating/added/viewed, genre categories |
| **View Modes** | Poster grid, list view, detail view, hero view |
| **Drag & Drop** | Limited (watchlist management) |
| **Keyboard Shortcuts** | Media playback shortcuts, navigation shortcuts |
| **Context Menus** | Right-click media actions, share, mark played |
| **Theming** | Dark theme, customizable home screen sections |
| **Responsive** | Web + mobile + TV + desktop apps, responsive layouts |
| **Accessibility** | Keyboard navigation, screen reader support, descriptive audio |
| **Offline** | Sync for offline viewing (Plex Pass), local playback |
| **Collaboration** | Shared libraries, managed users, Plex Home |
| **Media Handling** | Transcoding, HDR, Dolby Atmos, subtitle support, live TV, DVR, metadata matching |
| **Performance** | Hardware transcoding, adaptive streaming, DLNA |
| **Error Handling** | Server health monitoring, connection recovery, media analysis |
| **Onboarding** | Setup wizard, Plex Knowledge Base, community forums |
| **Customization** | Library management, metadata agents, plugins, DVR scheduling, Plexamp for music |

---

## 14. Jellyfin

| Category | Features |
|----------|----------|
| **Navigation** | Library sidebar, home screen, search, collections, playlists |
| **Item Management** | Libraries (movies, TV, music, books, photos), collections, playlists |
| **Search & Filtering** | Global search, genre/year/rating filters, similar items |
| **Sorting** | Sort by title/date/rating/added, genre categories |
| **View Modes** | Poster grid, list view, detail view, album view |
| **Drag & Drop** | Limited (playlist management) |
| **Keyboard Shortcuts** | Media playback shortcuts, navigation |
| **Context Menus** | Right-click media actions, mark played/unplayed |
| **Theming** | Multiple themes, dark/light mode, customizable |
| **Responsive** | Web + Android + iOS + TV + desktop + Kodi clients |
| **Accessibility** | Keyboard navigation, screen reader support |
| **Offline** | Download for offline viewing |
| **Collaboration** | Multi-user support, parental controls, shared libraries |
| **Media Handling** | Transcoding, hardware acceleration, subtitle support, live TV, DVR, photo management, book reader |
| **Performance** | Hardware transcoding, async scanning, streaming optimization |
| **Error Handling** | Server health monitoring, log system, plugin error isolation |
| **Onboarding** | Documentation, community forum, getting started guide |
| **Customization** | Plugin system, metadata providers, CSS theming, API access, open source extensibility |

---

## Cross-Application Feature Matrix

| Feature | Nautilus | Dolphin | Finder | TC | DC | Notion | Obsidian | Linear | Raycast | Craft | Lightroom | digiKam | Plex | Jellyfin |
|---------|----------|---------|--------|----|----|--------|----------|--------|---------|-------|-----------|---------|------|----------|
| **Dual Pane** | - | ✓ | - | ✓ | ✓ | - | - | - | - | - | - | - | - | - |
| **Tabs** | ✓ | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ | - | - | - | - | - | - |
| **Kanban** | - | - | - | - | - | ✓ | Plugin | ✓ | - | - | - | - | - | - |
| **Timeline** | - | - | - | - | - | ✓ | - | ✓ | - | - | ✓ | ✓ | - | - |
| **Graph View** | - | - | - | - | - | - | ✓ | - | - | - | - | - | - | - |
| **Real-time Collab** | - | - | - | - | - | ✓ | Limited | ✓ | - | ✓ | - | - | - | - |
| **Offline** | ✓ | ✓ | ✓ | ✓ | ✓ | Limited | ✓ | Limited | ✓ | ✓ | ✓ | ✓ | Limited | Limited |
| **Mobile App** | - | - | ✓ | ✓ | - | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | - | ✓ | ✓ |
| **Plugin System** | ✓ | ✓ | - | ✓ | ✓ | - | ✓ | ✓ | ✓ | - | ✓ | ✓ | ✓ | ✓ |
| **AI Features** | - | - | - | - | - | ✓ | Plugin | ✓ | ✓ | ✓ | ✓ | ✓ | - | - |

---

*Research compiled July 15, 2026*
